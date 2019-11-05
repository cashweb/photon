use std::pin::Pin;

use futures::{
    channel::{mpsc, oneshot},
    prelude::*,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::net::jsonrpc_client::*;

#[derive(Debug)]
pub enum BitcoinError {
    Client(ClientError),
    Decoding(hex::FromHexError),
    NoActiveTip,
}

impl From<ClientError> for BitcoinError {
    fn from(err: ClientError) -> Self {
        BitcoinError::Client(err)
    }
}

impl From<hex::FromHexError> for BitcoinError {
    fn from(err: hex::FromHexError) -> Self {
        BitcoinError::Decoding(err)
    }
}

#[derive(Deserialize)]
pub struct ChainTipStatus {
    height: u32,
    hash: String,
    status: String,
}

#[derive(Clone)]
pub struct ChainTip {
    pub height: u32,
    pub hash: Vec<u8>,
}

type QueueItem = Pin<Box<dyn Future<Output = Result<Response, ClientError>> + Send + 'static>>;
type CallbackSender = oneshot::Sender<Result<Response, ClientError>>;

#[derive(Clone)]
pub struct RequestQueue {
    sender: mpsc::Sender<(QueueItem, oneshot::Sender<Result<Response, ClientError>>)>,
}

impl RequestQueue {
    /// Create a active request queue with given capacity
    pub async fn with_capacity(capacity: usize) -> Self {
        // Construct request queue
        let (sender, recv) = mpsc::channel::<(QueueItem, CallbackSender)>(capacity);

        // Pop from queue and send result through callback oneshot
        let broker =
            recv.for_each(|(request, callback)| async { callback.send(request.await).unwrap() });
        tokio::spawn(broker);

        RequestQueue { sender }
    }

    /// Push an item to the request queue, awaits response
    pub async fn push(
        &self,
        request: QueueItem,
    ) -> Result<Result<Response, ClientError>, mpsc::TrySendError<(QueueItem, CallbackSender)>>
    {
        let (finish, callback) = oneshot::channel();
        self.sender.clone().try_send((request, finish))?;
        Ok(callback.await.unwrap())
    }
}

#[derive(Clone)]
pub struct BitcoinClient(Arc<JsonClient>);

impl BitcoinClient {
    pub fn new(endpoint: String, username: String, password: String) -> BitcoinClient {
        BitcoinClient(Arc::new(JsonClient::new(endpoint, username, password)))
    }

    /// Broadcast transaction to Bitcoin network, returns transaction ID
    pub async fn broadcast_tx(&self, raw_tx: &[u8]) -> Result<Vec<u8>, BitcoinError> {
        let request = self.0.build_request(
            "sendrawtransaction".to_string(),
            vec![Value::String(hex::encode(raw_tx))],
        );
        let tx_id_hex = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;
        Ok(hex::decode(tx_id_hex)?)
    }

    /// Get raw transaction from Bitcoin network
    pub async fn raw_tx(&self, tx_id: &[u8; 32]) -> Result<Vec<u8>, BitcoinError> {
        // Reverse tx_id before hex encoding
        trace!("fetching raw transaction with id {}...", hex::encode(tx_id));
        let request = self.0.build_request(
            "getrawtransaction".to_string(),
            vec![Value::String(hex::encode(tx_id))],
        );
        let raw_tx_hex = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;
        Ok(hex::decode(raw_tx_hex)?)
    }

    /// Get block hash from height
    pub async fn block_hash(&self, height: u32) -> Result<Vec<u8>, BitcoinError> {
        trace!("fetching block from height {}...", height);
        let request = self.0.build_request(
            "getblockhash".to_string(),
            vec![Value::Number(height.into())],
        );
        let block_hash_hex = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;
        Ok(hex::decode(block_hash_hex)?)
    }

    /// Get block from block hash
    pub async fn block(&self, block_hash: &[u8]) -> Result<Vec<u8>, BitcoinError> {
        trace!("fetching block with hash {}...", hex::encode(block_hash));
        let request = self.0.build_request(
            "getblock".to_string(),
            vec![Value::String(hex::encode(block_hash))],
        );
        let block_hex = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;
        Ok(hex::decode(block_hex)?)
    }

    /// Get block from height
    pub async fn block_from_height(&self, height: u32) -> Result<Vec<u8>, BitcoinError> {
        trace!("fetching block hash from height {}...", height);
        // Get block hash
        let request = self.0.build_request(
            "getblockhash".to_string(),
            vec![Value::Number(height.into())],
        );
        let block_hash_hex = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;

        // Get block
        trace!("fetching block with hash {}...", block_hash_hex);
        let request = self.0.build_request(
            "getblock".to_string(),
            vec![Value::String(block_hash_hex), Value::Bool(false)],
        );
        let block_hex = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;
        Ok(hex::decode(block_hex)?)
    }

    /// Get current chain tip
    pub async fn chain_tip(&self) -> Result<ChainTip, BitcoinError> {
        trace!("fetching current chain tip...");
        let request = self.0.build_request("getchaintips".to_string(), vec![]);
        let chain_tips = self
            .0
            .send_request(&request)
            .await?
            .into_result::<Vec<ChainTipStatus>>()?;
        let tip_status = chain_tips
            .iter()
            .find(|tip| tip.status == "active".to_string())
            .ok_or(BitcoinError::NoActiveTip)?;
        Ok(ChainTip {
            height: tip_status.height,
            hash: hex::decode(&tip_status.hash)?,
        })
    }

    /// Get block count
    pub async fn block_count(&self) -> Result<u32, BitcoinError> {
        trace!("fetching block count...");
        let request = self.0.build_request("getblockcount".to_string(), vec![]);
        Ok(self.0.send_request(&request).await?.into_result::<u32>()?)
    }

    /// Create a stream of raw blocks between heights [start, end)
    pub fn raw_block_stream<'a>(
        &'a self,
        start: u32,
        end: u32,
    ) -> impl Stream<Item = Result<(u32, Vec<u8>), BitcoinError>> + 'a {
        let stream = stream::iter(start..end).then(move |height| {
            self.block_from_height(height)
                .map(move |raw_block_res| raw_block_res.map(move |raw_block| (height, raw_block)))
        });
        Box::pin(stream)
    }
}

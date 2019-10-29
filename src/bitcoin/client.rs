use std::sync::Arc;

use serde_json::Value;

use crate::net::jsonrpc_client::*;

enum BitcoinError {
    Client(ClientError),
    Decoding(hex::FromHexError),
}

impl From<ClientError> for BitcoinError {
    fn from(err: ClientError) -> BitcoinError {
        BitcoinError::Client(err)
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
        let response_str = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;
        hex::decode(response_str).map_err(BitcoinError::Decoding)
    }

    /// Get raw transaction from Bitcoin network
    pub async fn raw_tx(&self, tx_id: &[u8]) -> Result<Vec<u8>, BitcoinError> {
        let request = self.0.build_request(
            "getrawtransaction".to_string(),
            vec![Value::String(hex::encode(tx_id))],
        );
        let response_str = self
            .0
            .send_request(&request)
            .await?
            .into_result::<String>()?;
        hex::decode(response_str).map_err(BitcoinError::Decoding)
    }
}

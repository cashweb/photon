use std::convert::TryInto;

use tonic::{Code, Request, Response, Status};

use crate::{
    bitcoin::client::*,
    db::{CachedOption, Database},
    net::{jsonrpc_client::ClientError, transaction::model::TransactionResponse},
};

pub mod model {
    tonic::include_proto!("transaction");
}

const INVALID_TX_ID_MSG: &str = "invalid transaction id";

#[derive(Clone)]
pub struct TransactionService {
    pub bitcoin_client: BitcoinClient,
    pub db: Database,
}

#[tonic::async_trait]
impl model::server::Transaction for TransactionService {
    async fn broadcast(
        &self,
        request: Request<model::BroadcastRequest>,
    ) -> Result<Response<model::BroadcastResponse>, Status> {
        let raw_tx = request.into_inner().raw_tx;
        let tx_id = self
            .bitcoin_client
            .broadcast_tx(&raw_tx)
            .await
            .map_err(|err| match err {
                BitcoinError::Client(err_inner) => match err_inner {
                    // An RPC error indicates either insufficient relay fee or a double spend attempt
                    // TODO: More detail here
                    ClientError::Rpc(_) => Status::new(Code::FailedPrecondition, String::new()),
                    // Any non-rpc error indicates an internal error
                    _ => Status::new(Code::Internal, String::new()),
                },
                // Else internal errors
                _ => Status::new(Code::Internal, String::new()), // Don't reveal internal server error
            })?;
        let reply = model::BroadcastResponse { tx_id };
        Ok(Response::new(reply))
    }

    async fn transaction(
        &self,
        request: Request<model::TransactionRequest>,
    ) -> Result<Response<model::TransactionResponse>, Status> {
        let request_inner = request.into_inner();

        let tx_id: [u8; 32] = request_inner.tx_id[..]
            .try_into()
            .map_err(|_| Status::new(Code::InvalidArgument, INVALID_TX_ID_MSG.to_string()))?;

        match self
                .db
                .get_tx(&tx_id)
                .map_err(|_| Status::new(Code::Internal, String::new()))? // Don't reveal internal server error
            {
                CachedOption::Some(mut tx_response) => {
                    // Get tx from bitcoind
                    let raw_tx = self.bitcoin_client.raw_tx(&tx_id).await.map_err(|_| {
                        // TODO: Handle missing transaction correctly
                        Status::new(Code::NotFound, String::new())})?;

                    // Insert missing raw_tx into response
                    tx_response.raw_tx = raw_tx;

                    // Insert cached tx response into DB
                    // TODO: On seperate thread? Log response?
                    self.db.clone().put_tx(&tx_id, &tx_response).unwrap_or(());

                    // TODO: Is it faster to mutate or reallocate?
                    if !request_inner.merkle {
                        tx_response = TransactionResponse {
                            raw_tx: tx_response.raw_tx,
                            ..Default::default()
                        }
                    }

                    Ok(Response::new(tx_response))
                }
                CachedOption::SomeCached(tx_response) => {
                    Ok(Response::new(tx_response))
                }
                CachedOption::None => Err(Status::new(Code::NotFound, String::new())),
            }
    }
}

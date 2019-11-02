use tonic::{Code, Request, Response, Status};

use crate::{bitcoin::client::*, net::jsonrpc_client::ClientError};

pub mod model {
    tonic::include_proto!("transaction");
}

#[derive(Clone)]
pub struct TransactionService {
    pub bitcoin_client: BitcoinClient,
}

#[tonic::async_trait]
impl model::server::Transaction for TransactionService {
    async fn broadcast(
        &self,
        request: Request<model::BroadcastRequest>,
    ) -> Result<Response<model::BroadcastResponse>, Status> {
        let raw_tx = request.into_inner().raw_tx;
        let tx_hash = self
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
                _ => Status::new(Code::Internal, String::new()),
            })?;
        let reply = model::BroadcastResponse { tx_hash };
        Ok(Response::new(reply))
    }

    // async fn transaction(
    //     &self,
    //     request: Request<model::TransactionRequest>,
    // ) -> Result<Response<model::TransactionResponse>, Status> {
    //     let request_inner = request.intoto_inner();
    //     if request_inner.merkle {
    //         // TODO: Grab merkle branch
    //         unreachable!()
    //     } else {
    //     }
    // }
}

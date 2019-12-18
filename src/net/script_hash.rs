pub mod model {
    tonic::include_proto!("script_hash");
}

use std::{convert::TryInto, pin::Pin, sync::Arc};

use futures::prelude::*;
use tonic::{Code, Request, Response, Status};

use crate::{db::Database, MEMPOOL};
use model::{history_response::MempoolItem, SubscribeRequest, SubscribeResponse};

type Sub = bus_queue::Subscriber<([u8; 32], [u8; 32])>;

const INVALID_SCRIPT_HASH_MSG: &str = "invalid script hash";

#[derive(Clone)]
pub struct ScriptHashService {
    db: Database,
    script_hash_bus: Sub,
}

impl ScriptHashService {
    pub fn new(db: Database, script_hash_bus: Sub) -> Self {
        ScriptHashService {
            db,
            script_hash_bus,
        }
    }
}

#[tonic::async_trait]
impl model::script_hash_server::ScriptHash for ScriptHashService {
    type SubscribeStream =
        Pin<Box<dyn Stream<Item = Result<SubscribeResponse, Status>> + Send + Sync + 'static>>;

    async fn history(
        &self,
        request: Request<model::HistoryRequest>,
    ) -> Result<Response<model::HistoryResponse>, Status> {
        let request_inner = request.into_inner();

        // Parse script hash
        let script_hash: [u8; 32] = request_inner.script_hash[..]
            .try_into()
            .map_err(|_| Status::new(Code::InvalidArgument, INVALID_SCRIPT_HASH_MSG.to_string()))?;

        let mempool_items: Vec<MempoolItem> = if request_inner.include_mempool_items {
            let ids: Vec<[u8; 32]> = MEMPOOL
                .lock()
                .await
                .get_transactions_ids(&script_hash)
                .unwrap_or(vec![]);

            ids.iter()
                .map(|tx_id| MempoolItem {
                    tx_id: tx_id.to_vec(),
                    fee: 0,
                    has_unconfirmed_parent: false,
                })
                .collect()
        } else {
            // TODO: Implement historic script hash
            return Err(Status::new(Code::Unimplemented, String::new()));
        };

        let reply = model::HistoryResponse {
            confirmed_items: vec![], // TODO: Implement historic script hash
            confirmed_continuation: None,
            mempool_items,
        };
        Ok(Response::new(reply))
    }

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let request_inner = request.into_inner();
        let request_script_hash = Arc::new(request_inner.script_hash);

        let response_stream = self.script_hash_bus.clone().filter_map(move |arc_val| {
            let request_script_hash_inner = request_script_hash.clone();
            async move {
                let (script_hash, status) = arc_val.as_ref();

                if request_script_hash_inner[..] == script_hash[..] {
                    Some(Ok(SubscribeResponse {
                        confirmed_status: vec![],
                        unconfirmed_status: status.to_vec(),
                    }))
                } else {
                    None
                }
            }
        });
        let pinned = Box::pin(response_stream) as Self::SubscribeStream;
        Ok(Response::new(pinned))
    }
}

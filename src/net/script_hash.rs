pub mod model {
    tonic::include_proto!("script_hash");
}

use std::{pin::Pin, sync::Arc};

use futures::prelude::*;
use tonic::{Code, Request, Response, Status};

use crate::{db::Database, zmq::HandlerError};
use model::{SubscribeRequest, SubscribeResponse};

type Sub = bus_queue::Subscriber<Result<([u8; 32], [u8; 32]), HandlerError>>;

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
impl model::server::ScriptHash for ScriptHashService {
    type SubscribeStream =
        Pin<Box<dyn Stream<Item = Result<SubscribeResponse, Status>> + Send + Sync + 'static>>;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let request_inner = request.into_inner();
        let request_script_hash = Arc::new(request_inner.script_hash);

        let response_stream = self.script_hash_bus.clone().filter_map(move |arc_val| {
            let request_script_hash_inner = request_script_hash.clone();
            async move {
                arc_val
                    .as_ref()
                    .as_ref()
                    .map(move |(script_hash, status)| {
                        if &request_script_hash_inner[..] == &script_hash[..] {
                            Some(SubscribeResponse {
                                confirmed_status: vec![],
                                unconfirmed_status: status.to_vec(),
                            })
                        } else {
                            None
                        }
                    })
                    .map_err(|err| Status::new(Code::Aborted, format!("{:?}", err)))
                    .transpose()
            }
        });
        let pinned = Box::pin(response_stream) as Self::SubscribeStream;
        Ok(Response::new(pinned))
    }
}

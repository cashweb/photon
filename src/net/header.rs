pub mod model {
    tonic::include_proto!("header");
}

use std::pin::Pin;

use bus_queue::Subscriber as BusSubscriber;
use futures::prelude::*;
use tonic::{Code, Request, Response, Status};

use crate::{bitcoin::client::*, db::Database};
use model::{HeadersResponse, SubscribeResponse};

#[derive(Clone)]
pub struct HeaderService {
    bitcoin_client: BitcoinClient,
    db: Database,
    header_bus: BusSubscriber<(u32, [u8; 80])>,
}

impl HeaderService {
    pub fn new(
        bitcoin_client: BitcoinClient,
        db: Database,
        header_bus: BusSubscriber<(u32, [u8; 80])>,
    ) -> Self {
        HeaderService {
            bitcoin_client,
            db,
            header_bus,
        }
    }
}

#[tonic::async_trait]
impl model::server::Header for HeaderService {
    type SubscribeStream =
        Pin<Box<dyn Stream<Item = Result<SubscribeResponse, Status>> + Send + Sync + 'static>>;

    async fn headers(
        &self,
        request: Request<model::HeadersRequest>,
    ) -> Result<Response<model::HeadersResponse>, Status> {
        let request_inner = request.into_inner();

        let headers = self
            .db
            .get_headers(request_inner.start_height, request_inner.count)
            .map_err(|_| Status::new(Code::Internal, String::new()))?; // Don't reveal internal server error
        let header_response = HeadersResponse {
            headers,
            ..Default::default()
        };
        Ok(Response::new(header_response))
    }

    // TODO
    async fn subscribe(
        &self,
        request: Request<()>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let response_stream = self
            .header_bus
            .clone()
            .map(move |arc_val| arc_val.as_ref().clone())
            .map(|(height, header)| {
                Ok(SubscribeResponse {
                    height: height,
                    header: header.to_vec(),
                })
            });
        let pinned = Box::pin(response_stream) as Self::SubscribeStream;
        Ok(Response::new(pinned))
    }
}

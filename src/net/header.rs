pub mod model {
    tonic::include_proto!("header");
}

use std::pin::Pin;

use futures::Stream;
use tonic::{Code, Request, Response, Status};

use crate::{bitcoin::client::*, db::Database};
use model::{HeadersResponse, SubscribeResponse};

#[derive(Clone)]
pub struct HeaderService {
    pub bitcoin_client: BitcoinClient,
    pub db: Database,
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
        Err(Status::new(Code::Unavailable, String::new()))
    }
}

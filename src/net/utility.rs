use clap::crate_version;
use tonic::{Request, Response, Status};

use crate::SETTINGS;

pub mod model {
    tonic::include_proto!("utility");
}

const AGENT: &str = "";

#[derive(Clone)]
pub struct UtilityService {}

#[tonic::async_trait]
impl model::server::Utility for UtilityService {
    async fn banner(
        &self,
        _request: Request<()>,
    ) -> Result<Response<model::BannerResponse>, Status> {
        let reply = model::BannerResponse {
            banner: SETTINGS.banner.clone(),
        };
        Ok(Response::new(reply))
    }

    async fn donation_address(
        &self,
        _request: Request<()>,
    ) -> Result<Response<model::DonationAddressResponse>, Status> {
        let reply = model::DonationAddressResponse {
            address: SETTINGS.donation_address.clone(),
        };
        Ok(Response::new(reply))
    }

    async fn ping(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn version(
        &self,
        _request: Request<()>,
    ) -> Result<Response<model::VersionResponse>, Status> {
        let reply = model::VersionResponse {
            agent: AGENT.to_string(),
            version: crate_version!().to_string(),
        };
        Ok(Response::new(reply))
    }
}

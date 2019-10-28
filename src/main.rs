#[macro_use]
extern crate lazy_static;

use clap::{crate_authors, crate_description, crate_version, App, Arg, ArgMatches};
use tonic::{transport::Server, Request, Response, Status};

pub mod utility {
    tonic::include_proto!("utility");
}
pub mod transaction {
    tonic::include_proto!("transaction");
}
pub mod settings;

const AGENT: &str = "";

lazy_static! {
    // Declare APP and get matches
    static ref CLI_ARGS: ArgMatches<'static> = App::new("Photon")
        .version(crate_version!())
        .author(crate_authors!("/n"))
        .about(crate_description!())
        .arg(Arg::with_name("bind")
            .long("bind")
            .short("b")
            .help("Sets server bind address")
            .takes_value(true))
        .get_matches();

    // Fetch settings
    static ref SETTINGS: settings::Settings = settings::Settings::fetch().unwrap();
}

#[derive(Default)]
pub struct Daemon {}

#[tonic::async_trait]
impl utility::server::Utility for Daemon {
    async fn banner(
        &self,
        _request: Request<()>,
    ) -> Result<Response<utility::BannerResponse>, Status> {
        let reply = utility::BannerResponse {
            banner: SETTINGS.banner.clone(),
        };
        Ok(Response::new(reply))
    }

    async fn donation_address(
        &self,
        _request: Request<()>,
    ) -> Result<Response<utility::DonationAddressResponse>, Status> {
        let reply = utility::DonationAddressResponse {
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
    ) -> Result<Response<utility::VersionResponse>, Status> {
        let reply = utility::VersionResponse {
            agent: AGENT.to_string(),
            version: crate_version!().to_string(),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SETTINGS.bind.parse().unwrap();
    println!("starting server @ {}", addr);
    let daemon = Daemon::default();
    Server::builder()
        .serve(addr, utility::server::UtilityServer::new(daemon))
        .await?;

    Ok(())
}

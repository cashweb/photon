#[macro_use]
extern crate lazy_static;

use std::sync::Arc;

use clap::{crate_authors, crate_description, crate_version, App, Arg, ArgMatches};
use tonic::transport::Server;

use crate::{
    bitcoin::client::BitcoinClient,
    net::{router::Router, transaction, utility},
};

pub mod bitcoin;
pub mod net;
pub mod settings;

lazy_static! {
    // Declare APP and get matches
    static ref CLI_ARGS: ArgMatches<'static> = App::new("Photon")
        .version(crate_version!())
        .author(crate_authors!("/n"))
        .about(crate_description!())
        .arg(Arg::with_name("bitcoin-rpc")
            .long("bitcoin-rpc")
            .help("Sets the Bitcoin RPC address"))
        .arg(Arg::with_name("bitcoin-user")
            .long("bitcoin-user")
            .help("Sets the Bitcoin RPC user")
            .takes_value(true))
        .arg(Arg::with_name("bitcoin-password")
            .long("bitcoin-password")
            .help("Sets the Bitcoin RPC password")
            .takes_value(true))
        .arg(Arg::with_name("bind")
            .long("bind")
            .short("b")
            .help("Sets server bind address")
            .takes_value(true))
        .arg(Arg::with_name("banner")
            .long("banner")
            .help("Sets server banner")
            .takes_value(true))
        .arg(Arg::with_name("donation-address")
            .long("donation-address")
            .help("Sets donation address")
            .takes_value(true))
        .get_matches();

    // Fetch settings
    static ref SETTINGS: settings::Settings = settings::Settings::fetch().unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SETTINGS.bind.parse().unwrap();
    println!("starting server @ {}", addr);

    // Init Bitcoin client
    let bitcoin_client = BitcoinClient::new(
        SETTINGS.bitcoin_rpc.clone(),
        SETTINGS.bitcoin_user.clone(),
        SETTINGS.bitcoin_password.clone(),
    );

    // Construct utility service
    let utility_service = utility::UtilityService {};

    // Construct transaction service
    let transaction_service = transaction::TransactionService { bitcoin_client };

    // Aggregate services using router
    // TODO: Replace when routing is natively supported
    let router = Router {
        utility_service: Arc::new(utility_service),
        transaction_service: Arc::new(transaction_service),
    };
    Server::builder().serve(addr, router).await?;

    Ok(())
}

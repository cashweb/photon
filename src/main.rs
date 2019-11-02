#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

pub mod bitcoin;
pub mod db;
pub mod net;
pub mod settings;
pub mod state;

use std::sync::Arc;

use clap::{crate_authors, crate_description, crate_version, App, Arg, ArgMatches};
use futures::prelude::*;
use tonic::transport::Server;

use crate::{
    bitcoin::{
        block_processing::process_block_stream,
        client::{BitcoinClient, BitcoinError},
    },
    net::{router::Router, transaction, utility},
};
use db::Database;
use state::StateMananger;

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
        .arg(Arg::with_name("db-path")
            .short("d")
            .long("db-path")
            .help("Sets the database path")
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

    // Init state manager
    static ref STATE_MANAGER: StateMananger = StateMananger::default();
}

#[derive(Debug)]
enum SyncError {
    Chaintip(BitcoinError),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init logging
    env_logger::init();

    // Init Bitcoin client
    let bitcoin_client = BitcoinClient::new(
        SETTINGS.bitcoin_rpc.clone(),
        SETTINGS.bitcoin_user.clone(),
        SETTINGS.bitcoin_password.clone(),
    );

    // Init Database
    let db = Database::try_new(&SETTINGS.db_path).expect("failed to open database");

    // Start syncing
    let bitcoin_client_inner = bitcoin_client.clone();
    let sync = async move {
        info!("starting synchronization...");

        // Get current chain height
        let block_count = bitcoin_client_inner
            .block_count()
            .await
            .map_err(SyncError::Chaintip)?;

        info!("current chain height: {}", block_count);

        // Get oldest valid block
        // TODO: Scan database for this
        let earliest_valid_height = 0;

        let raw_block_stream =
            bitcoin_client_inner.raw_block_stream(earliest_valid_height, block_count);

        let process_blocks = process_block_stream(raw_block_stream, db);

        process_blocks.into_future().await;
        Ok::<(), SyncError>(())
    };

    if let Err(err) = sync.await {
        println!("{:?}", err);
    };

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

    // Start server
    let addr = SETTINGS.bind.parse().unwrap();
    info!("starting server @ {}", addr);
    Server::builder().serve(addr, router).await?;

    Ok(())
}

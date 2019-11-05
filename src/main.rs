#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

pub mod bitcoin;
pub mod db;
pub mod net;
pub mod settings;
pub mod state;
pub mod synchronization;

use bitcoin_zmq::ZMQListener;
use clap::{crate_authors, crate_description, crate_version, App, Arg, ArgMatches};
use futures::{future::try_join, prelude::*};
use tonic::transport::{Error as TonicError, Server};

use crate::{
    bitcoin::client::BitcoinClient,
    net::{
        header::{model::server::HeaderServer, HeaderService},
        transaction::{model::server::TransactionServer, TransactionService},
        utility::{model::server::UtilityServer, UtilityService},
    },
};
use db::Database;
use state::StateMananger;
use synchronization::{synchronize, SyncingError};

lazy_static! {
    // Declare APP and get matches
    static ref CLI_ARGS: ArgMatches<'static> = App::new("Photon")
        .version(crate_version!())
        .author(crate_authors!("/n"))
        .about(crate_description!())
        .arg(Arg::with_name("bitcoin")
            .long("bitcoin")
            .help("Sets the Bitcoin address")
            .takes_value(true))
        .arg(Arg::with_name("bitcoin-rpc-port")
            .long("bitcoin-rpc-port")
            .help("Sets the Bitcoin RPC port")
            .takes_value(true))
        .arg(Arg::with_name("bitcoin-zmq-port")
            .long("bitcoin-zmq-port")
            .help("Sets the Bitcoin ZMQ port")
            .takes_value(true))
        .arg(Arg::with_name("bitcoin-tls")
            .long("bitcoin-tls")
            .help("Use TLS to connect to bitcoind"))
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
        .arg(Arg::with_name("resync")
            .short("r")
            .long("resync")
            .help("Resynchronise the server from scratch"))
        .arg(Arg::with_name("sync-from")
            .long("sync-from")
            .help("Resynchronise the server from given height")
            .takes_value(true)
            .conflicts_with("resync"))
        .get_matches();

    // Fetch settings
    static ref SETTINGS: settings::Settings = settings::Settings::fetch().unwrap();

    // Init state manager
    static ref STATE_MANAGER: StateMananger = StateMananger::default();
}

#[derive(Debug)]
enum AppError {
    Syncing(SyncingError),
    ServerError(TonicError),
    MistypedCLI(String),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Init logging
    env_logger::init();

    // Init Bitcoin client
    let protocol = if SETTINGS.bitcoin_tls {
        "https"
    } else {
        "http"
    };
    let bitcoin_client = BitcoinClient::new(
        format!(
            "{}://{}:{}",
            protocol,
            SETTINGS.bitcoin.clone(),
            SETTINGS.bitcoin_rpc_port.clone()
        ),
        SETTINGS.bitcoin_user.clone(),
        SETTINGS.bitcoin_password.clone(),
    );

    // Setup ZMQ listener
    // let listener = ZMQListener

    // Init Database
    let db = Database::try_new(&SETTINGS.db_path).expect("failed to open database");

    let sync_opt = if let Some(arg) = CLI_ARGS.value_of("sync-from") {
        if let Ok(from) = arg.parse::<u32>() {
            Some(from)
        } else {
            return Err(AppError::MistypedCLI(
                "`sync-from` must be an unsigned 32-bit integer".to_string(),
            ));
        }
    } else {
        if CLI_ARGS.is_present("resync") {
            Some(0)
        } else {
            None
        }
    };
    let sync = synchronize(bitcoin_client.clone(), db.clone(), sync_opt);

    // Construct header service
    let header_svc = HeaderServer::new(HeaderService {
        bitcoin_client: bitcoin_client.clone(),
        db: db.clone(),
    });

    // Construct utility service
    let utility_svc = UtilityServer::new(UtilityService {});

    // Construct transaction service
    let transaction_svc = TransactionServer::new(TransactionService { bitcoin_client, db });

    // Start server
    let addr = SETTINGS.bind.parse().unwrap();
    info!("starting server @ {}", addr);
    let server = Server::builder()
        .add_service(header_svc)
        .add_service(utility_svc)
        .add_service(transaction_svc)
        .serve(addr);

    try_join(
        server.map_err(AppError::ServerError),
        sync.map_err(AppError::Syncing),
    )
    .await?;

    Ok(())
}

use bitcoin::{
    consensus::encode::Error as DecodeError, util::psbt::serialize::Deserialize, Transaction,
};
use bitcoin_zmq::{SubscriptionError, ZMQError, ZMQListener};
use futures::prelude::*;

use crate::{
    bitcoin::{block_processing::*, tx_processing::script_hash_transaction},
    db::Database,
    STATE_MANAGER,
};

type HeaderBus = bus_queue::Publisher<Result<(u32, [u8; 80]), HandlerError>>;
type ScriptHashBus = bus_queue::Publisher<Result<[u8; 32], HandlerError>>;

#[derive(Debug)]
pub enum MempoolError {
    TxDecode(DecodeError),
    Subscription(SubscriptionError),
}

#[derive(Debug)]
pub enum HandlerError {
    Block(BlockProcessingError),
    Mempool(MempoolError),
    Connection(ZMQError),
    Broker,
    Increment(rocksdb::Error),
}

impl From<ZMQError> for HandlerError {
    fn from(err: ZMQError) -> Self {
        HandlerError::Connection(err)
    }
}

impl From<rocksdb::Error> for HandlerError {
    fn from(err: rocksdb::Error) -> Self {
        HandlerError::Increment(err)
    }
}

pub async fn handle_zmq(
    block_addr: &str,
    tx_addr: &str,
    db: Database,
    mut header_bus: HeaderBus,
    mut script_hash_bus: ScriptHashBus,
) -> Result<(), HandlerError> {
    // Bind
    let block_listener = ZMQListener::bind(block_addr).await?;
    let tx_listener = ZMQListener::bind(tx_addr).await?;

    // Handle blocks
    let mut tx_stream = Box::pin(
        tx_listener
            .stream()
            .map_err(MempoolError::Subscription)
            .and_then(move |raw_tx| {
                async move {
                    let tx = Transaction::deserialize(&raw_tx).map_err(MempoolError::TxDecode)?;
                    Ok(stream::iter(
                        script_hash_transaction(&tx).into_iter().map(|tx| Ok(tx)),
                    ))
                }
            })
            .try_flatten()
            .map_err(HandlerError::Mempool),
    );

    // Broadcast to all subscribers
    let broadcast_tx = script_hash_bus
        .send_all(&mut tx_stream)
        .map_err(|_| HandlerError::Broker);

    // Pair blocks with sync position
    let paired_raw_block_stream = block_listener.stream().map_ok(|raw_block| {
        let height = STATE_MANAGER.sync_position();
        (height, raw_block)
    });

    // Decode raw block stream
    let paired_block_stream = decode_block_stream(paired_raw_block_stream);

    // TODO: Validate block header here

    // Process stream of decoded blocks
    let db_inner = db.clone();
    let process_block_stream =
        process_block_stream(paired_block_stream, db_inner).map_err(HandlerError::Block);

    // Record and log progress
    let mut increment = Box::pin(
        process_block_stream.and_then(move |(block_height, header)| {
            let db_inner = db.clone();
            async move {
                info!("processed block {}", block_height);

                let position = STATE_MANAGER.increment_sync_position();

                // Cache result periodically
                trace!("stored sync position {}", position);
                db_inner.set_sync_position(position + 1)?;
                Ok((block_height, header))
            }
        }),
    );

    // Broadcast to all subscribers
    let broadcast_block = header_bus
        .send_all(&mut increment)
        .map_err(|_| HandlerError::Broker);

    future::try_join(broadcast_tx, broadcast_block)
        .map(|_| ())
        .await;
    Ok(())
}

use bitcoin_zmq::{ZMQError, ZMQListener};
use futures::prelude::*;

use crate::{bitcoin::block_processing::*, db::Database, STATE_MANAGER};

type Bus = bus_queue::Publisher<Result<(u32, [u8; 80]), HandlerError>>;

#[derive(Debug)]
pub enum HandlerError {
    Block(BlockProcessingError),
    Mempool,
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
    mut header_bus: Bus,
) -> Result<(), HandlerError> {
    // Bind
    let block_listener = ZMQListener::bind(block_addr).await?;
    let tx_listener = ZMQListener::bind(tx_addr).await?;

    // Handle blocks
    let tx_handler = tx_listener
        .stream()
        .try_for_each(move |raw| {
            println!("raw tx: {:?}", hex::encode(raw));
            future::ok(())
        })
        .map_err(|_| HandlerError::Mempool);

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
    let broadcast = header_bus
        .send_all(&mut increment)
        .map_err(|_| HandlerError::Broker);

    Ok(future::try_join(tx_handler, broadcast).map(|_| ()).await)
}

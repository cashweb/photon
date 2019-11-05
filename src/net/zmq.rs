use bitcoin_zmq::{ZMQError, ZMQListener};
use futures::prelude::*;

use crate::{bitcoin::block_processing::*, db::Database, STATE_MANAGER};

#[derive(Debug)]
pub enum HandlerError {
    Block(BlockProcessingError),
    Mempool,
    Connection(ZMQError),
}

impl From<ZMQError> for HandlerError {
    fn from(err: ZMQError) -> Self {
        HandlerError::Connection(err)
    }
}

pub async fn handle_zmq(block_addr: &str, tx_addr: &str, db: Database) -> Result<(), HandlerError> {
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
        .map_err(|err| HandlerError::Mempool);

    // Handle blocks
    let db_inner = db.clone();
    let block_callback = |block_height| {
        info!("processed block {}", block_height);

        let position = STATE_MANAGER.increment_sync_position();

        // Cache result periodically
        trace!("stored sync position {}", position);
        db_inner.set_sync_position(position + 1)?;
        Ok(())
    };
    let paired_block_stream = block_listener.stream().map_ok(|raw_block| {
        let height = STATE_MANAGER.sync_position();
        (height, raw_block)
    });
    let block_handler = par_process_block_stream(paired_block_stream, db, &block_callback)
        .map_err(|err| HandlerError::Block(err));
    Ok(future::try_join(tx_handler, block_handler)
        .map(|_| ())
        .await)
}

use futures::TryFutureExt;

use crate::{
    bitcoin::{block_processing::*, client::*},
    db::Database,
};

#[derive(Debug)]
pub enum SyncingError {
    Chaintip(BitcoinError),
    BlockProcessing(BlockProcessingError),
}

impl From<BlockProcessingError> for SyncingError {
    fn from(err: BlockProcessingError) -> Self {
        SyncingError::BlockProcessing(err)
    }
}

fn block_callback(block_height: u32) {
    if block_height % 1_000 == 0 {
        info!("processed block {}", block_height);
    }
}

pub async fn synchronize(bitcoin_client: BitcoinClient, db: Database) -> Result<(), SyncingError> {
    info!("starting synchronization...");

    // Get current chain height
    let block_count = bitcoin_client
        .block_count()
        .map_err(SyncingError::Chaintip)
        .await?;

    info!("current chain height: {}", block_count);

    // Get oldest valid block
    // TODO: Scan database for this
    let earliest_valid_height: u32 = 0;
    info!("earliest valid height: {}...", earliest_valid_height);

    let raw_block_stream = bitcoin_client.raw_block_stream(earliest_valid_height, block_count);

    // Begin processing blocks
    info!(
        "processing blocks {} to {}...",
        earliest_valid_height, block_count
    );

    let result = par_process_block_stream(raw_block_stream, db, &block_callback)
        .map_err(SyncingError::BlockProcessing)
        .await?;

    info!("completed synchronization");
    Ok(result)
}

use futures::TryFutureExt;

use crate::{
    bitcoin::{block_processing::*, client::*},
    db::Database,
    STATE_MANAGER,
};

const PERSIST_SYNC_POS_INTERVAL: u32 = 128;

#[derive(Debug)]
pub enum SyncingError {
    /// Error grabbing chain tip
    Chaintip(BitcoinError),
    /// Error grabbing last sync position
    LastSyncGet(rocksdb::Error),
    /// Error setting last sync position
    LastSyncSet(rocksdb::Error),
    /// Error during block processing
    BlockProcessing(BlockProcessingError),
}

impl From<BlockProcessingError> for SyncingError {
    fn from(err: BlockProcessingError) -> Self {
        SyncingError::BlockProcessing(err)
    }
}

pub async fn synchronize(
    bitcoin_client: BitcoinClient,
    db: Database,
    resync: Option<u32>,
) -> Result<(), SyncingError> {
    info!("starting synchronization...");

    // Get current chain height
    let block_count = bitcoin_client
        .block_count()
        .map_err(SyncingError::Chaintip)
        .await?;

    info!("current chain length: {}", block_count);

    // Get oldest valid block
    let last_sync_position: u32 = match resync {
        Some(some) => some,
        None => match db.get_sync_position() {
            Ok(opt) => match opt {
                Some(some) => {
                    STATE_MANAGER.set_sync_position(some);
                    some
                }
                None => 0,
            },
            Err(err) => return Err(SyncingError::LastSyncGet(err)),
        },
    };

    if last_sync_position == block_count {
        info!("already up-to-date");
        return Ok(());
    }

    // TODO: Validate from this position?

    // Construct block stream
    let raw_block_stream = bitcoin_client.raw_block_stream(last_sync_position, block_count);

    // Begin processing blocks
    info!(
        "processing blocks {} to {}...",
        last_sync_position, block_count
    );

    let db_inner = db.clone();
    let block_callback = |block_height| {
        if block_height % 1_000 == 0 {
            info!("processed block {}", block_height);
        }
        let position = STATE_MANAGER.increment_sync_position();

        // Cache result periodically
        if position % PERSIST_SYNC_POS_INTERVAL == 0 {
            trace!("stored sync position {}", position);
            db_inner.set_sync_position(position + 1)?;
        }
        Ok(())
    };

    par_process_block_stream(raw_block_stream, db, &block_callback)
        .map_err(SyncingError::BlockProcessing)
        .await?;

    // Finalize state position
    let position = STATE_MANAGER.sync_position();
    info!("setting position: {}", position);
    db_inner
        .set_sync_position(position)
        .map_err(SyncingError::LastSyncSet)?;

    // TODO: Check that we've actually met the chaintip here
    // Perhaps just simply recurse
    info!("completed synchronization");
    Ok(())
}

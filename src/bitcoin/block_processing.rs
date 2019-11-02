use bitcoin::{consensus::encode::Decodable, consensus::encode::Error as ConsensusError, Block};
use futures::prelude::*;

use super::{client::*, tx_processing::*};
use crate::db::Database;

const BLOCK_PROCESSING_CONCURRENCY: usize = 8;

/// Create a stream of raw blocks between heights [start, end)
pub fn raw_block_stream(
    start: u32,
    end: u32,
    client: &'static BitcoinClient,
) -> impl Stream<Item = Result<(u32, Vec<u8>), BitcoinError>> + Send {
    stream::iter(start..end)
        .map(|height| Ok(height))
        .and_then(move |height| {
            client
                .block_from_height(height)
                .map(move |raw_block_res| raw_block_res.map(|raw_block| (height, raw_block)))
        })
}

pub enum BlockProcessingError {
    Bitcoin(BitcoinError),
    BlockDecoding(ConsensusError),
    Transaction(TxProcessingError),
}

impl From<TxProcessingError> for BlockProcessingError {
    fn from(err: TxProcessingError) -> Self {
        BlockProcessingError::Transaction(err)
    }
}

impl From<BitcoinError> for BlockProcessingError {
    fn from(err: BitcoinError) -> Self {
        BlockProcessingError::Bitcoin(err)
    }
}

impl From<ConsensusError> for BlockProcessingError {
    fn from(err: ConsensusError) -> Self {
        BlockProcessingError::BlockDecoding(err)
    }
}

pub async fn process_block_stream(
    raw_block_stream: impl Stream<Item = Result<(u32, Vec<u8>), BitcoinError>> + Send + 'static,
    db: Database,
) -> Result<(), BlockProcessingError> {
    let block_stream = raw_block_stream.map(|res| {
        let (height, raw_block) = res?;
        let block = Block::consensus_decode(&raw_block[..])?;
        Ok((height, block))
    });

    // Main processing loop
    let processing = block_stream.try_for_each_concurrent(
        BLOCK_PROCESSING_CONCURRENCY,
        move |(block_height, block)| {
            // Process transactions
            let txs = block.txdata;
            process_transactions(block_height, txs, db.clone()).map_err(|err| err.into())
        },
    );
    processing.await
}

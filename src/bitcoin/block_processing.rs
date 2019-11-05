use bitcoin::{
    consensus::encode::Error as ConsensusError,
    consensus::encode::{Decodable, Encodable},
    Block,
};
use futures::{future::join_all, prelude::*};
use rocksdb::Error as RocksError;

use super::{client::*, tx_processing::*};
use crate::db::Database;

const BLOCK_CHUNK_SIZE: usize = 128;

#[derive(Debug)]
pub enum BlockProcessingError {
    Bitcoin(BitcoinError),
    BlockDecoding(ConsensusError),
    Transaction(TxProcessingError),
    Database(RocksError),
}

impl From<TxProcessingError> for BlockProcessingError {
    fn from(err: TxProcessingError) -> Self {
        BlockProcessingError::Transaction(err)
    }
}

impl From<ConsensusError> for BlockProcessingError {
    fn from(err: ConsensusError) -> Self {
        BlockProcessingError::BlockDecoding(err)
    }
}

impl From<RocksError> for BlockProcessingError {
    fn from(err: RocksError) -> Self {
        BlockProcessingError::Database(err)
    }
}

impl From<BitcoinError> for BlockProcessingError {
    fn from(err: BitcoinError) -> Self {
        BlockProcessingError::Bitcoin(err)
    }
}

pub async fn process_block(
    height: u32,
    block: Block,
    db: Database,
    block_callback: &dyn Fn(u32) -> Result<(), BlockProcessingError>,
) -> Result<(), BlockProcessingError> {
    // Process header
    let mut raw_header: [u8; 80] = [0; 80];
    block.header.consensus_encode(&mut raw_header[..]).unwrap();
    db.put_header(height, &raw_header)?;

    // Do some action dependending on block height
    block_callback(height)?;

    // Process transactions
    let txs = block.txdata;
    Ok(process_transactions(height, txs, db).await?)
}

pub async fn par_process_block_stream(
    raw_block_stream: impl Stream<Item = Result<(u32, Vec<u8>), BitcoinError>> + Send,
    db: Database,
    block_callback: &dyn Fn(u32) -> Result<(), BlockProcessingError>,
) -> Result<(), BlockProcessingError> {
    // Split stream into chunks
    let block_stream = raw_block_stream
        .err_into::<BlockProcessingError>()
        .and_then(|(height, raw_block): (u32, Vec<u8>)| {
            async move {
                let block = Block::consensus_decode(&raw_block[..])?;
                Ok((height, block))
            }
        });

    let processing = block_stream.try_for_each_concurrent(
        BLOCK_CHUNK_SIZE,
        move |(height, block): (u32, Block)| {
            let db_inner = db.clone();
            process_block(height, block, db_inner, block_callback)
        },
    );
    processing.await
}

use bitcoin::{
    consensus::encode::Error as ConsensusError,
    consensus::encode::{Decodable, Encodable},
    Block,
};
use futures::prelude::*;
use rocksdb::Error as RocksError;

use super::{client::*, tx_processing::*};
use crate::db::Database;

#[derive(Debug)]
pub enum BlockProcessingError {
    Bitcoin(BitcoinError),
    BlockDecoding(ConsensusError),
    Transaction(TxProcessingError),
    Database(RocksError),
    Subscription(bitcoin_zmq::SubscriptionError),
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

impl From<bitcoin_zmq::SubscriptionError> for BlockProcessingError {
    fn from(err: bitcoin_zmq::SubscriptionError) -> Self {
        BlockProcessingError::Subscription(err)
    }
}

pub async fn process_block(
    height: u32,
    block: Block,
    db: Database,
) -> Result<(u32, [u8; 80]), BlockProcessingError> {
    // Process header
    let mut raw_header: [u8; 80] = [0; 80];
    block.header.consensus_encode(&mut raw_header[..]).unwrap();
    db.put_header(height, &raw_header)?;

    // Process transactions
    let txs = block.txdata;
    process_transactions(height, &txs, db).await?;

    Ok((height, raw_header))
}

pub fn decode_block_stream<E: Into<BlockProcessingError>>(
    raw_block_stream: impl Stream<Item = Result<(u32, Vec<u8>), E>> + Send,
) -> impl Stream<Item = Result<(u32, Block), BlockProcessingError>> + Send {
    raw_block_stream
        .err_into::<BlockProcessingError>()
        .and_then(|(height, raw_block): (u32, Vec<u8>)| {
            async move {
                let block = Block::consensus_decode(&raw_block[..])?;
                Ok((height, block))
            }
        })
}

pub fn process_block_stream<E: Into<BlockProcessingError>>(
    block_stream: impl TryStream<Ok = (u32, Block), Error = E> + Send,
    db: Database,
) -> impl TryStream<Ok = (u32, [u8; 80]), Error = BlockProcessingError> + Send {
    block_stream.err_into::<BlockProcessingError>().and_then(
        move |(height, block): (u32, Block)| {
            let db_inner = db.clone();
            process_block(height, block, db_inner)
        },
    )
}

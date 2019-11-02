use std::future::Future;

use bitcoin::Transaction;
use bitcoin_hashes::Hash;
use futures::channel::mpsc;
use rocksdb::Error as RocksError;

use crate::db::{model::TransactionEntry, Database};

pub enum TxProcessingError {
    Database(RocksError),
}

impl From<RocksError> for TxProcessingError {
    fn from(err: RocksError) -> Self {
        TxProcessingError::Database(err)
    }
}

pub async fn process_transactions(
    block_height: u32,
    txs: Vec<Transaction>,
    db: Database,
) -> Result<(), TxProcessingError> {
    Ok(txs.iter().enumerate().try_for_each(move |(pos, tx)| {
        let tx_id = tx.txid().into_inner();

        // Construct transaction entry
        let data = TransactionEntry {
            raw_tx: vec![],
            block_height,
            merkle: vec![],
            pos: pos as u32,
        };
        db.put_tx(&tx_id, &data)
    })?)
}

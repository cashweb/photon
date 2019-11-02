use bitcoin::Transaction;
use bitcoin_hashes::Hash;
use rocksdb::Error as RocksError;

use crate::{
    db::Database,
    net::transaction::model::{transaction_response::TxMerkleInfo, TransactionResponse},
};

#[derive(Debug)]
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
        trace!("processing tx {}...", hex::encode(tx_id));

        // Construct transaction entry
        let data = TransactionResponse {
            raw_tx: vec![], // Do not cache raw transaction during sync
            merkle: Some(TxMerkleInfo {
                block_height,
                merkle: vec![],
                pos: pos as u32,
            }),
        };
        db.put_tx(&tx_id, &data)
    })?)
}

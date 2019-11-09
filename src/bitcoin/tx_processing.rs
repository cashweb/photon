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
    txs: &[Transaction],
    db: Database,
) -> Result<(), TxProcessingError> {
    Ok(txs.iter().enumerate().try_for_each(move |(pos, tx)| {
        let tx_id_rev = tx.txid().into_inner();
        let mut tx_id: [u8; 32] = [0; 32];
        for (i, byte) in tx_id_rev.iter().rev().enumerate() {
            tx_id[i] = *byte;
        }
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

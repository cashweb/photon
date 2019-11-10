use std::collections::HashMap;

use bitcoin::Transaction;
use bitcoin_hashes::{sha256d::Hash as Sha256d, Hash};

pub struct Mempool {
    pool: HashMap<[u8; 32], Transaction>,
    status: HashMap<[u8; 32], Vec<[u8; 32]>>,
}

impl Default for Mempool {
    fn default() -> Self {
        Mempool {
            pool: HashMap::with_capacity(1024),
            status: HashMap::with_capacity(2048),
        }
    }
}

impl Mempool {
    pub fn put_transaction(&mut self, tx_id: &[u8; 32], tx: Transaction) {
        self.pool.insert(*tx_id, tx);
    }

    pub fn get_transaction_by_id(&self, tx_id: &[u8; 32]) -> Option<Transaction> {
        self.pool.get(tx_id).cloned()
    }

    pub fn get_transactions_ids(&self, script_hash: &[u8; 32]) -> Option<Vec<[u8; 32]>> {
        self.status.get(script_hash).cloned()
    }

    pub fn get_transactions_by_script_hash(
        &self,
        script_hash: &[u8; 32],
    ) -> Option<Vec<Transaction>> {
        self.status.get(script_hash).map(|tx_ids| {
            tx_ids
                .iter()
                .map(|tx_id| self.get_transaction_by_id(tx_id).unwrap()) // This is safe
                .collect()
        })
    }

    // Append script to status and return new status
    pub fn append_status(&mut self, script_hash: &[u8; 32], tx_id: &[u8; 32]) -> [u8; 32] {
        // TODO: Check whether tx already exists in mempool?
        // This may be needed before we get eviction notices
        match self.status.get_mut(script_hash) {
            Some(ids) => {
                // Insert into sorted vector
                if let Err(pos) = ids.binary_search(&tx_id) {
                    ids.insert(pos, *tx_id);
                }
                let concat = ids.concat();
                Sha256d::hash(&concat).into_inner()
            }
            None => {
                self.status.insert(*script_hash, vec![*tx_id]);
                Sha256d::hash(tx_id).into_inner()
            }
        }
    }

    pub fn get_status(&self, script_hash: &[u8; 32]) -> Option<[u8; 32]> {
        self.status.get(script_hash).map(|ids| {
            let concat = ids.concat();
            Sha256d::hash(&concat).into_inner()
        })
    }

    pub fn flush(&mut self) {
        *self = Mempool::default()
    }
}

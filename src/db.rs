pub mod model {
    tonic::include_proto!("database");
}

use std::sync::Arc;

use prost::Message;
use rocksdb::{Error, Options, DB};

use crate::net::transaction::model::TransactionResponse;

const TX_ID_PREFIX_LEN: usize = 8;

#[derive(Clone)]
pub struct Database(Arc<DB>);

pub enum CachedOption<T> {
    /// Value found but no cache
    Some(T),
    /// Value found with cache
    SomeCached(T),
    /// Value not found
    None,
}

impl Database {
    pub fn try_new(path: &str) -> Result<Self, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        DB::open(&opts, &path).map(Arc::new).map(Database)
    }

    pub fn put_tx(&self, tx_id: &[u8; 32], data: &TransactionResponse) -> Result<(), Error> {
        let mut raw = Vec::with_capacity(data.encoded_len());
        data.encode(&mut raw).unwrap();
        self.0.put(&tx_id[..TX_ID_PREFIX_LEN], raw)
    }

    pub fn get_tx(&self, tx_id: &[u8; 32]) -> Result<CachedOption<TransactionResponse>, Error> {
        self.0.get(&tx_id[..TX_ID_PREFIX_LEN]).map(|opt| match opt {
            Some(some) => {
                // TODO: Use wrapping metadata protobuf
                let tx_entry = TransactionResponse::decode(some.as_ref()).unwrap();

                if tx_entry.raw_tx.is_empty() {
                    CachedOption::Some(tx_entry)
                } else {
                    CachedOption::SomeCached(tx_entry)
                }
            }
            None => CachedOption::None,
        })
    }
}

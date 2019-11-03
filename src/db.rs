pub mod model {
    tonic::include_proto!("database");
}

use std::{convert::TryInto, sync::Arc};

use prost::Message;
use rocksdb::{Error, Options, DB};

use crate::net::transaction::model::TransactionResponse;

// Values larger than 32 will cause panics
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
        // Encode transaction response
        let mut raw = Vec::with_capacity(data.encoded_len());
        data.encode(&mut raw).unwrap();

        // Prefix key
        let mut key: [u8; TX_ID_PREFIX_LEN + 1] = [0; TX_ID_PREFIX_LEN + 1];
        key[0] = b't';
        for i in 0..TX_ID_PREFIX_LEN {
            key[i + 1] = tx_id[i]
        }

        self.0.put(&key, raw)
    }

    pub fn get_tx(&self, tx_id: &[u8; 32]) -> Result<CachedOption<TransactionResponse>, Error> {
        // Prefix key
        let mut key: [u8; TX_ID_PREFIX_LEN + 1] = [0; TX_ID_PREFIX_LEN + 1];
        key[0] = b't';
        for i in 0..TX_ID_PREFIX_LEN {
            key[i + 1] = tx_id[i]
        }

        self.0.get(&key).map(|opt| match opt {
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

    pub fn set_sync_position(&self, position: u32) -> Result<(), Error> {
        let key: [u8; 1] = [b's'];
        let bytes = position.to_le_bytes();
        self.0.put(&key[..], bytes)
    }

    pub fn get_sync_position(&self) -> Result<Option<u32>, Error> {
        let key: [u8; 1] = [b's'];
        self.0.get(&key[..]).map(|res| {
            res.map(|val| {
                // This panics is record is malformed
                let bytes: [u8; 4] = val.as_ref().try_into().unwrap();
                u32::from_le_bytes(bytes)
            })
        })
    }
}

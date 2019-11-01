pub mod model {
    tonic::include_proto!("database");
}

use std::sync::Arc;

use prost::Message;
use rocksdb::{Error, Options, DB};

use model::*;

const TX_ID_PREFIX_LEN: usize = 8;

#[derive(Clone)]
pub struct Database(Arc<DB>);

impl Database {
    pub fn try_new(path: &str) -> Result<Self, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        DB::open(&opts, &path).map(Arc::new).map(Database)
    }

    pub fn put_tx(
        &self,
        tx_id: &[u8; TX_ID_PREFIX_LEN],
        data: &TransactionEntry,
    ) -> Result<(), Error> {
        let mut raw = Vec::with_capacity(data.encoded_len());
        data.encode(&mut raw).unwrap();
        self.0.put(&tx_id, raw)
    }
}

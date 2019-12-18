pub mod header;
pub mod jsonrpc_client;
pub mod script_hash;
pub mod transaction;
pub mod utility;
pub mod zmq;

use tonic::{Code, Status};

pub use crate::state::BarrierError;

impl Into<Status> for BarrierError {
    fn into(self) -> Status {
        match self {
            BarrierError::Syncing => Status::new(Code::Unavailable, "server syncing".to_string()),
            BarrierError::ReOrgOverflow => Status::new(
                Code::ResourceExhausted,
                "request expelled during reorg".to_string(),
            ),
        }
    }
}

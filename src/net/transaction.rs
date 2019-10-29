pub mod model {
    tonic::include_proto!("transaction");
}

pub struct TransactionService {}

#[tonic::async_trait]
impl model::server::Transaction for TransactionService {}

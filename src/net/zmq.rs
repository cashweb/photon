use bitcoin_zmq::{errors::SubscriptionError, ZMQListener};
use futures::prelude::*;

pub async fn handle_zmq(block_addr: &str, tx_addr: &str) -> Result<(), SubscriptionError> {
    // Bind
    let block_listener = ZMQListener::bind(block_addr).await?;
    let tx_listener = ZMQListener::bind(tx_addr).await?;

    // Handle blocks
    let tx_handler = tx_listener.stream().try_for_each(move |raw| {
        println!("raw tx: {:?}", hex::encode(raw));
        future::ok(())
    });

    // Handle blocks
    let block_handler = block_listener.stream().try_for_each(move |raw| {
        println!("raw block: {:?}", hex::encode(raw));
        future::ok(())
    });
    Ok(future::try_join(tx_handler, block_handler)
        .map(|_| ())
        .await)
}

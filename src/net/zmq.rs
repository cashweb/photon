use bitcoin::consensus::encode::Encodable;
use bitcoin_zmq::{ZMQError, ZMQListener};
use bus_queue::{Publisher as BusPublisher, Subscriber as BusSubscriber};
use futures::prelude::*;
use futures::sink::SinkExt;

use crate::{bitcoin::block_processing::*, db::Database, BROADCAST_CAPACITY, STATE_MANAGER};

#[derive(Debug)]
pub enum HandlerError {
    Block(BlockProcessingError),
    Mempool,
    Connection(ZMQError),
    Broker(std::sync::mpsc::SendError<(u32, [u8; 80])>),
}

impl From<ZMQError> for HandlerError {
    fn from(err: ZMQError) -> Self {
        HandlerError::Connection(err)
    }
}

pub async fn handle_zmq(
    block_addr: &str,
    tx_addr: &str,
    db: Database,
    header_pub: BusPublisher<(u32, [u8; 80])>,
    header_sub: BusSubscriber<(u32, [u8; 80])>,
) -> Result<(), HandlerError> {
    // Bind
    let block_listener = ZMQListener::bind(block_addr).await?;
    let tx_listener = ZMQListener::bind(tx_addr).await?;

    // Handle blocks
    let tx_handler = tx_listener
        .stream()
        .try_for_each(move |raw| {
            println!("raw tx: {:?}", hex::encode(raw));
            future::ok(())
        })
        .map_err(|err| HandlerError::Mempool);

    // Handle blocks
    let db_inner = db.clone();
    let block_callback = |block_height| {
        info!("processed block {}", block_height);

        let position = STATE_MANAGER.increment_sync_position();

        // Cache result periodically
        trace!("stored sync position {}", position);
        db_inner.set_sync_position(position + 1)?;
        Ok(())
    };
    let paired_raw_block_stream = block_listener.stream().map_ok(|raw_block| {
        let height = STATE_MANAGER.sync_position();
        (height, raw_block)
    });

    // Decode raw block stream
    let paired_block_stream = decode_block_stream(paired_raw_block_stream);

    // TODO: Validate block header here

    // Feed into broadcast channel
    // TODO: Re-evaluate this, it's a mess
    let (relay_sender, recv) = futures::channel::mpsc::channel(BROADCAST_CAPACITY);
    let relay_block_stream = paired_block_stream.and_then(move |(height, block)| {
        let mut relay_sender_inner = relay_sender.clone();
        async move {
            let mut header: [u8; 80] = [0; 80];
            block.header.consensus_encode(&mut header[..]).unwrap(); // TODO: Make this safe
            let height_inner = height.clone();
            relay_sender_inner.send((height_inner, header)).await;
            Ok((height, block))
        }
    });
    let broadcast = recv
        .map(|x| Ok(x))
        .forward(header_pub)
        .map_err(|err| HandlerError::Broker(err));

    let block_handler = process_block_stream(relay_block_stream, db, &block_callback)
        .map_err(|err| HandlerError::Block(err));
    Ok(future::try_join3(tx_handler, block_handler, broadcast)
        .map(|_| ())
        .await)
}

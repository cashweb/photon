use bitcoin::{consensus::encode::Decodable, Block};
use futures::prelude::*;

use super::client::*;

pub fn raw_block_stream(
    start: u32,
    end: u32,
    client: &'static BitcoinClient,
) -> impl Stream<Item = Result<Vec<u8>, BitcoinError>> {
    stream::iter(start..end)
        .map(|height| Ok(height))
        .and_then(move |height| client.block_from_height(height))
}

pub fn process_block_stream(
    raw_block_stream: impl Stream<Item = Result<Vec<u8>, BitcoinError>>,
) -> Result<(), BitcoinError> {
    let block_stream = raw_block_stream.filter_map(|raw_block_res| {
        async move {
            let raw_block = match raw_block_res {
                Ok(raw_block) => raw_block,
                Err(err) => {
                    // TODO: Log and retry
                    unreachable!()
                }
            };

            match Block::consensus_decode(&raw_block[..]) {
                Ok(block) => Some(block),
                Err(err) => {
                    // TODO: Log and retry
                    unreachable!()
                }
            }
        }
    });
    Ok(())
}

use crate::{
    block::Block, translator::TranslatorConfig, types::translator_types::NameToAddressCache,
};
use alloy::primitives::FixedBytes;
use antelope::api::client::{APIClient, DefaultProvider};
use eyre::{eyre, Context, Result};
use hex::encode;
use std::str::FromStr;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use tracing::{debug, error, info};

// BlocksShipped represents a struct that is persisted on disk and
// represents a blocks that was shipped after the finalized block
struct BlocksShipped {
    block: Block,
    last_block_number: u32,
}

pub async fn final_processor(
    db: DBWithThreadMode<SingleThreaded>,
    config: TranslatorConfig,
    api_client: APIClient<DefaultProvider>,
    mut rx: mpsc::Receiver<Block>,
    tx: Option<mpsc::Sender<(FixedBytes<32>, Block)>>,
    stop_tx: oneshot::Sender<()>,
) -> Result<()> {
    let mut last_log = Instant::now();
    let mut unlogged_blocks = 0;
    let mut unlogged_transactions = 0;

    let mut parent_hash = FixedBytes::from_str(&config.prev_hash)
        .wrap_err("Prev hash config is not a valid 32 byte hex string")?;

    let validate_hash = match config.validate_hash {
        Some(hash) => Some(
            FixedBytes::from_str(&hash)
                .wrap_err("Validate hash config is not a valid 32 byte hex string")?,
        ),
        None => None,
    };

    let mut validated = validate_hash.is_none();

    let native_to_evm_cache = NameToAddressCache::new(api_client);
    let stop_block = config.stop_block.unwrap_or(u32::MAX);

    while let Some(mut block) = rx.recv().await {
        if block.block_num > stop_block {
            break;
        }
        debug!("Finalizing block #{}", block.block_num);

        // let lib = block.result.last_irreversible;
        // if let Some(b) = db.get(block.block_num)? {
            info!("Block already exists. {}", block.block_num);
        // } else {
            let block_shiped = BlocksShipped {
                block: block.clone(),
                last_block_number: block.block_num.clone(),
            };
            info!("Persist a block in a db. {} ", block.block_num);
            // db.put(block.block_num.clone(), block_shiped).expect("Cannot persist");
        // }


        let header = block
            .generate_evm_data(parent_hash, config.block_delta, &native_to_evm_cache)
            .await;

        unlogged_blocks += 1;
        unlogged_transactions += block.transactions.len();

        let block_hash = header.hash_slow();

        if !validated {
            if let Some(validate_hash) = validate_hash {
                validated = validate_hash == block_hash;
                if !validated {
                    error!(
                        "Initial hash validation failed!, expected: \"{validate_hash}\" got: \"{block_hash}\"",
                    );
                    error!("Header: {:#?}", header);
                    return Err(eyre!("Initial hash validation failed!"));
                }
            }
        }

        if last_log.elapsed().as_secs_f64() > 1.0 {
            let blocks_sec = unlogged_blocks as f64 / last_log.elapsed().as_secs_f64();
            let trx_sec = unlogged_transactions as f64 / last_log.elapsed().as_secs_f64();
            info!(
                "Block #{} 0x{} - processed {} blocks/sec and {} tx/sec",
                block.block_num,
                encode(block_hash),
                blocks_sec,
                trx_sec
            );
            //info!("Block map is {} long", block_map.len());
            unlogged_blocks = 0;
            unlogged_transactions = 0;
            last_log = Instant::now();
        }
        // TODO: Fork handling, hashing, all the things...

        let block_num = block.block_num;
        if let Some(tx) = tx.clone() {
            if let Err(error) = tx.send((block_hash, block)).await {
                error!("Failed to send finished block to exit stream!! {error}.");
                break;
            }
        }
        parent_hash = block_hash;
        if block_num == stop_block {
            debug!("Processed stop block #{block_num}, exiting...");
            stop_tx
                .send(())
                .map_err(|_| eyre!("Can't send stop message"))?;
            break;
        }
    }
    while rx.recv().await.is_some() {}
    info!("Exiting final processor...");
    Ok(())
}

use std::{str::FromStr, sync::Arc};

use alloy::primitives::FixedBytes;
use antelope::api::client::{APIClient, DefaultProvider};
use eyre::{eyre, Context, Result};
use hex::encode;
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::Instant,
};
use tracing::{debug, error, info};

use crate::{
    block::{DecodedRow, ProcessingEVMBlock, TelosEVMBlock},
    data::{Block, Chain, Database},
    translator::TranslatorConfig,
    types::translator_types::NameToAddressCache,
};

pub async fn final_processor(
    config: TranslatorConfig,
    api_client: APIClient<DefaultProvider>,
    mut rx: mpsc::Receiver<ProcessingEVMBlock>,
    tx: Option<mpsc::Sender<TelosEVMBlock>>,
    stop_tx: oneshot::Sender<()>,
    chain: Arc<Mutex<Chain>>,
    db: Database,
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

            unlogged_blocks = 0;
            unlogged_transactions = 0;
            last_log = Instant::now();
        }
        // TODO: Fork handling, hashing, all the things...

        let block_num = block.block_num;
        let transactions = block.transactions;
        let block: Block = Block::new(block.block_num, block_hash.to_string());

        let is_fork = {
            let mut chain = chain.lock().await;
            chain
                .add(block.clone())
                .expect("Forked can be added to the chain")
        };

        if is_fork {
            db.delete_from(block_num)
                .expect("Blocks can be deleted from the database");
        }

        db.put_block(block)
            .expect("Block can be put to the database");

        let completed_block = TelosEVMBlock {
            header,
            block_num,
            block_hash,
            transactions,

            new_revision: block.new_revision,
            new_gas_price: block.new_gas_price,
            new_wallets: block.new_wallets,
            account_rows: block
                .decoded_rows
                .iter()
                .filter_map(|r| {
                    if let DecodedRow::Account(row) = r {
                        Some(row.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            account_state_rows: block
                .decoded_rows
                .iter()
                .filter_map(|r| {
                    if let DecodedRow::AccountState(row) = r {
                        Some(row.clone())
                    } else {
                        None
                    }
                })
                .collect(),
	    is_fork,
        };

        if let Some(tx) = tx.clone() {
            if let Err(error) = tx.send(completed_block).await {
                error!("Failed to send finished block to exit stream!! {error}.");
                break;
            }
        }

        {
            let chain = chain.lock().await;
            let last_block = chain.last().expect("Last added block exists");

            parent_hash = FixedBytes::from_str(last_block.hash.as_str())?;
        }

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

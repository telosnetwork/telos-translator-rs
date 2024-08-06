use crate::block::Block;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};
use crate::types::translator_types::BlockOrSkip;

pub async fn evm_block_processor(mut block_rx: Receiver<Block>, block_tx: Sender<BlockOrSkip>) {
    while let Some(mut block) = block_rx.recv().await {
        if block.block_num % 1000 == 0 {
            info!(
                "Processing block {}, queue: {}",
                block.block_num,
                block_rx.len()
            );
        }
        block.deserialize();
        if block_tx.send(BlockOrSkip::Block(block)).await.is_err() {
            error!("Failed to send block to final processor!!");
            break;
        }
    }
}

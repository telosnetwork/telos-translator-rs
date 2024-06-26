use std::sync::Arc;
use std::thread::sleep;
use alloy::hex::FromHex;
use alloy::primitives::FixedBytes;
use antelope::api::client::{APIClient, DefaultProvider};
use dashmap::DashMap;
use crate::block::Block;
use crate::types::types::NameToAddressCache;

pub async fn evm_block_generator(start_block: u32, block_map: Arc<DashMap<u32, Block>>, api_client: APIClient<DefaultProvider>) {
    let mut block_num = start_block;
    let mut parent_hash = FixedBytes::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let native_to_evm_cache = NameToAddressCache::new(api_client);
    loop {
        if let Some(mut block) = block_map.get_mut(&block_num) {
            let header = block.generate_evm_data(parent_hash, &native_to_evm_cache).await;
            parent_hash = header.hash_slow();
            drop(block);
            block_map.remove(&block_num);
            block_num += 1;
        }
        sleep(std::time::Duration::from_millis(5));
    }
}

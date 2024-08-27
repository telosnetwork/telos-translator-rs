use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

use crate::{block, types::ship_types::BlockPosition};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Block {
    pub number: u32,
    pub hash: String,
}

impl Block {
    pub fn new(number: u32, hash: String) -> Self {
        Self { number, hash }
    }
}

impl From<BlockPosition> for Block {
    fn from(
        BlockPosition {
            block_num,
            block_id,
        }: BlockPosition,
    ) -> Self {
        Block {
            number: block_num,
            hash: block_id.to_string(),
        }
    }
}

impl From<block::ProcessingEVMBlock> for Block {
    fn from(value: block::ProcessingEVMBlock) -> Self {
        Self {
            number: value.block_num,
            hash: value.block_hash.to_string(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Chain {
    lib: Option<Block>,
    blocks: Vec<Block>,
}

impl Chain {
    /// Sets the new LIB.
    /// If LIB block, or blocks after the LIB, are processed, blocks before the LIB are discarded.
    /// Returns LIB if LIB is updated.
    /// Panics if user tries to set previous LIB or same LIB with different hash.
    pub fn set_lib(&mut self, lib: Block) -> Result<Option<&Block>> {
        if self.lib() == Some(&lib) {
            return Ok(None);
        }
        let Some(previous) = self.lib.take() else {
            self.lib = Some(lib);
            return Ok(self.lib());
        };

        if previous.number > lib.number {
            return Err(eyre!("Cannot set previous LIB"));
        }

        self.lib = Some(lib.clone());
        if let Some(last) = self.blocks.last() {
            if last.number >= lib.number {
                self.blocks = self
                    .blocks
                    .clone()
                    .into_iter()
                    .filter(|block| block.number >= lib.number)
                    .collect();
            }
        }
        Ok(self.lib())
    }

    /// Adds processed block.
    /// Returns error if block is not next block of the last processed block.
    pub fn add(&mut self, block: Block) -> Result<()> {
        if self.lib.is_none() {
            return Err(eyre!("Cannot add block if LIB is not set"));
        };

        if let Some(last) = self.blocks.last() {
            if block.number != last.number + 1 {
                return Err(eyre!(
                    "Block {} is not next of the block {}",
                    block.number,
                    last.number
                ));
            }
        }

        self.blocks.push(block);
        Ok(())
    }

    pub fn length(&self) -> usize {
        self.blocks.len()
    }

    pub fn lib(&self) -> Option<&Block> {
        self.lib.as_ref()
    }

    pub fn last(&self) -> Option<&Block> {
        self.blocks.last()
    }

    pub fn get(&self, block_num: u32) -> Option<&Block> {
        self.blocks.iter().find(|block| block.number == block_num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_block() {
        let lib0 = Block::new(0, "0".to_string());
        let lib4 = Block::new(4, "4".to_string());
        let block1 = Block::new(1, "1".to_string());
        let block2 = Block::new(2, "2".to_string());
        let block3 = Block::new(3, "3".to_string());
        let block4 = Block::new(4, "4".to_string());
        let block5 = Block::new(5, "5".to_string());
        let block6 = Block::new(6, "6".to_string());

        let mut chain = Chain::default();
        assert!(matches!(chain.set_lib(lib0), Ok(Some(_))));

        assert!(chain.add(block1.clone()).is_ok());
        assert!(chain.add(block3.clone()).is_err());
        assert!(chain.add(block2.clone()).is_ok());
        assert!(chain.add(block2.clone()).is_err());
        assert!(chain.add(block3.clone()).is_ok());

        assert_eq!(chain.length(), 3);

        assert!(chain.add(block4.clone()).is_ok());
        assert!(chain.add(block5.clone()).is_ok());
        assert!(chain.add(block6.clone()).is_ok());

        assert_eq!(chain.length(), 6);

        assert!(matches!(chain.set_lib(lib4), Ok(Some(_))));

        assert_eq!(chain.length(), 3);

        assert!(chain.add(block5).is_err());
        assert!(chain.add(block6).is_err());
    }
}

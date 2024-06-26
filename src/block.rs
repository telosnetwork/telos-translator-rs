use std::cmp::Ordering;
use alloy_consensus::constants::{EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};
use alloy_consensus::Header;
use antelope::chain::checksum::Checksum256;
use antelope::chain::Decoder;
use alloy::primitives::{Bytes, FixedBytes};

use tracing::info;
use crate::transaction::Transaction;

use crate::types::evm_types::{RawAction, PrintedReceipt, TransferAction, WithdrawAction};
use crate::types::ship_types::{ActionTrace, GetBlocksResultV0, SignedBlock, TableDelta, TransactionTrace};
use crate::types::names::*;
use crate::types::types::NameToAddressCache;



pub trait BasicTrace {
    fn action_name(&self) -> u64;
    fn action_account(&self) -> u64;
    fn receiver(&self) -> u64;
    fn console(&self) -> String;
    fn data(&self) -> Vec<u8>;
}

impl BasicTrace for ActionTrace {
    fn action_name(&self) -> u64 {
        match self {
            ActionTrace::V0(a) => a.act.name.n,
            ActionTrace::V1(a) => a.act.name.n,
        }
    }

    fn action_account(&self) -> u64 {
        match self {
            ActionTrace::V0(a) => a.act.account.n,
            ActionTrace::V1(a) => a.act.account.n,
        }
    }

    fn receiver(&self) -> u64 {
        match self {
            ActionTrace::V0(a) => a.receiver.n,
            ActionTrace::V1(a) => a.receiver.n,
        }
    }

    fn console(&self) -> String {
        match self {
            ActionTrace::V0(a) => a.console.clone(),
            ActionTrace::V1(a) => a.console.clone(),
        }
    }

    fn data(&self) -> Vec<u8> {
        match self {
            ActionTrace::V0(a) => a.act.data.clone(),
            ActionTrace::V1(a) => a.act.data.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub block_num: u64,
    pub block_hash: Checksum256,
    pub chain_id: u64,
    pub result: GetBlocksResultV0,
    pub signed_block: Option<SignedBlock>,
    pub block_traces: Option<Vec<TransactionTrace>>,
    pub block_deltas: Option<Vec<TableDelta>>,
    pub transactions: Vec<Transaction>,
}

pub fn decode_raw(raw: &[u8]) -> RawAction {
    let mut decoder = Decoder::new(raw);
    let raw = &mut RawAction::default();
    decoder.unpack(raw);
    raw.clone()
}

pub fn decode_transfer(raw: &[u8]) -> TransferAction {
    let mut decoder = Decoder::new(raw);
    let transfer = &mut TransferAction::default();
    decoder.unpack(transfer);
    transfer.clone()
}

pub fn decode_withdraw(raw: &[u8]) -> WithdrawAction {
    let mut decoder = Decoder::new(raw);
    let withdraw = &mut WithdrawAction::default();
    decoder.unpack(withdraw);
    withdraw.clone()
}

impl Block {
    pub fn new(
        chain_id: u64,
        block_num: u64,
        block_hash: Checksum256,
        result: GetBlocksResultV0
    ) -> Self {
        Self {
            block_num,
            block_hash,
            chain_id,
            result,
            signed_block: None,
            block_traces: None,
            block_deltas: None,
            transactions: vec![],
        }
    }

    pub fn deserialize(&mut self) {
        if let Some(b) = &self.result.block {
            let mut decoder = Decoder::new(b.as_slice());
            let signed_block = &mut SignedBlock::default();
            decoder.unpack(signed_block);
            self.signed_block = Some(signed_block.clone());
        }

        if let Some(t) = &self.result.traces {
            let mut decoder = Decoder::new(t.as_slice());
            let block_traces: &mut Vec<TransactionTrace> = &mut vec![];
            decoder.unpack(block_traces);
            self.block_traces = Some(block_traces.to_vec());
        }

        if let Some(d) = &self.result.deltas {
            let mut decoder = Decoder::new(d.as_slice());
            let block_deltas: &mut Vec<TableDelta> = &mut vec![];
            decoder.unpack(block_deltas);
            self.block_deltas = Some(block_deltas.to_vec());
        }
    }

    async fn handle_action(&mut self, action: Box<dyn BasicTrace + Send>, native_to_evm_cache: &NameToAddressCache) {
        let action_name = action.action_name();
        let action_account = action.action_account();
        let action_receiver = action.receiver();

        if action_account == EOSIO_EVM && action_name == RAW {
            // Normally signed EVM transaction
            let raw = decode_raw(&action.data());
            let printed_receipt = PrintedReceipt::from_console(action.console());
            if printed_receipt.is_none() {
                panic!("No printed receipt found for raw action in block: {}", self.block_num);
            }
            let transaction = Transaction::from_raw_action(self.chain_id, self.transactions.len(), self.block_hash, raw, printed_receipt.unwrap()).await;
            self.transactions.push(transaction);
        } else if action_account == EOSIO_EVM && action_name == WITHDRAW {
            // Withdrawal from EVM
            let withdraw_action = decode_withdraw(&action.data());
            let transaction = Transaction::from_withdraw(self.chain_id, self.transactions.len(), self.block_hash, withdraw_action, native_to_evm_cache).await;
            self.transactions.push(transaction);
        } else if action_account == EOSIO_TOKEN && action_name == TRANSFER && action_receiver == EOSIO_EVM {
            // Deposit/transfer to EVM
            let transfer_action = decode_transfer(&action.data());
            if SYSTEM_ACCOUNTS.contains(&transfer_action.from.n) {
                return;
            }

            let transaction = Transaction::from_transfer(self.chain_id, self.transactions.len(), self.block_hash, transfer_action, native_to_evm_cache).await;
            self.transactions.push(transaction.clone());
        } else if action_account == EOSIO_EVM && action_name == DORESOURCES {
            // TODO: Handle doresources action
        }
    }

    pub async fn generate_evm_data(&mut self, parent_hash: FixedBytes<32>, native_to_evm_cache: &NameToAddressCache) -> Header {
        if self.signed_block.is_none() || self.block_traces.is_none() || self.block_deltas.is_none() {
            panic!("Block::to_evm called on a block with missing data");
        }

        let traces = self.block_traces.clone().unwrap();

        for t in traces {
            match t {
                TransactionTrace::V0(t) => {
                    for action in t.action_traces {
                        self.handle_action(Box::new(action), &native_to_evm_cache).await;
                    }
                }
            }
        }

        Header {
            parent_hash,
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: Default::default(),
            state_root: EMPTY_ROOT_HASH,
            transactions_root: EMPTY_ROOT_HASH,
            receipts_root: EMPTY_ROOT_HASH,
            withdrawals_root: None,
            logs_bloom: Default::default(),
            difficulty: Default::default(),
            number: self.block_num,
            gas_limit: 0x7fffffff,
            gas_used: 0,
            timestamp: self.signed_block.clone().unwrap().header.header.timestamp as u64,
            mix_hash: Default::default(),
            nonce: Default::default(),
            base_fee_per_gas: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
            requests_root: None,
            extra_data: Bytes::from(self.block_hash.data),
        }
    }
}

impl Ord for Block {
    fn cmp(&self, other: &Self) -> Ordering {
        self.block_num.cmp(&other.block_num)
    }
}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.block_num == other.block_num
    }
}

impl Eq for Block {}

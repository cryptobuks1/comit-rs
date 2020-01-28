#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use self::contract_address::*;
pub use web3::types::{
    Address, Block, BlockId, BlockNumber, Bytes, Log, Transaction, TransactionReceipt,
    TransactionRequest, H160, H2048, H256, U128, U256,
};

mod contract_address;

#[derive(Debug, PartialEq)]
pub struct TransactionAndReceipt {
    pub transaction: Transaction,
    pub receipt: TransactionReceipt,
}

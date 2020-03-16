pub mod actions;
mod facade;
pub mod ledger;
pub mod ledger_states;
pub mod rfc003;
pub mod state;
pub mod swap_communication_state;
mod swap_error_state;
mod swap_id;

pub use self::{facade::*, swap_communication_state::*, swap_error_state::*, swap_id::*};

use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Copy,
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
)]
pub enum HashFunction {
    #[strum(serialize = "SHA-256")]
    #[serde(rename = "SHA-256")]
    Sha256,
}

#[derive(Debug, Clone, Copy)]
pub enum SwapProtocol {
    Rfc003(HashFunction),
}

#[derive(Clone, Copy, Debug, Display, EnumString, PartialEq)]
pub enum Role {
    Alice,
    Bob,
}

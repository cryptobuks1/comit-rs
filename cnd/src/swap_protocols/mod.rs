pub mod actions;
mod facade;
pub mod ledger;
pub mod ledger_states;
pub mod rfc003;
mod secret;
pub mod state;
pub mod swap_communication_states;
mod swap_error_states;
mod swap_id;

pub use self::{
    facade::*,
    ledger_states::*,
    secret::{FromErr, Secret, SecretHash},
    swap_communication_states::*,
    swap_error_states::*,
    swap_id::*,
};

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

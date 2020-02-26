use crate::swap_protocols::{
    rfc003::{create_swap::SwapEvent, ActorState, Ledger},
    swap_id::SwapId,
};
use std::{any::Any, cmp::Ordering, collections::HashMap, sync::Mutex};

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum Error {
    #[error("invalid type")]
    InvalidType,
}

#[allow(clippy::type_complexity)]
pub trait StateStore: Send + Sync + 'static {
    fn insert<A>(&self, key: SwapId, value: A)
    where
        A: ActorState;
    fn get<A>(&self, key: &SwapId) -> Result<Option<A>, Error>
    where
        A: ActorState;
    fn update<A>(
        &self,
        key: &SwapId,
        update: SwapEvent<
            <<A as ActorState>::AL as Ledger>::HtlcLocation,
            <<A as ActorState>::AL as Ledger>::Transaction,
            <<A as ActorState>::BL as Ledger>::HtlcLocation,
            <<A as ActorState>::BL as Ledger>::Transaction,
            A::AA,
            A::BA,
        >,
    ) where
        A: ActorState;
}

#[derive(Default, Debug)]
pub struct InMemoryStateStore {
    states: Mutex<HashMap<SwapId, Box<dyn Any + Send + Sync>>>,
}

impl StateStore for InMemoryStateStore {
    fn insert<A>(&self, key: SwapId, value: A)
    where
        A: ActorState,
    {
        let mut states = self.states.lock().unwrap();
        states.insert(key, Box::new(value));
    }

    fn get<A>(&self, key: &SwapId) -> Result<Option<A>, Error>
    where
        A: ActorState,
    {
        let states = self.states.lock().unwrap();
        match states.get(key) {
            Some(state) => match state.downcast_ref::<A>() {
                Some(state) => Ok(Some(state.clone())),
                None => Err(Error::InvalidType),
            },
            None => Ok(None),
        }
    }

    #[allow(clippy::type_complexity)]
    fn update<A>(
        &self,
        key: &SwapId,
        event: SwapEvent<
            <<A as ActorState>::AL as Ledger>::HtlcLocation,
            <<A as ActorState>::AL as Ledger>::Transaction,
            <<A as ActorState>::BL as Ledger>::HtlcLocation,
            <<A as ActorState>::BL as Ledger>::Transaction,
            A::AA,
            A::BA,
        >,
    ) where
        A: ActorState,
    {
        let mut actor_state = match self.get::<A>(key) {
            Ok(Some(actor_state)) => actor_state,
            Ok(None) => {
                tracing::warn!("Value not found for key {}", key);
                return;
            }
            Err(_invalid_type) => {
                tracing::warn!("Attempted to get state with wrong type for key {}", key);
                return;
            }
        };

        match event {
            SwapEvent::AlphaDeployed(deployed) => actor_state
                .alpha_ledger_mut()
                .transition_to_deployed(deployed),
            SwapEvent::AlphaFunded(funded) => {
                let expected_asset = actor_state.expected_alpha_asset();

                match expected_asset.cmp(&funded.asset) {
                    Ordering::Equal => actor_state.alpha_ledger_mut().transition_to_funded(funded),
                    _ => actor_state
                        .alpha_ledger_mut()
                        .transition_to_incorrectly_funded(funded),
                }
            }
            SwapEvent::AlphaRedeemed(redeemed) => {
                // what if redeemed.secret.hash() != secret_hash in request ??

                actor_state
                    .alpha_ledger_mut()
                    .transition_to_redeemed(redeemed);
            }
            SwapEvent::AlphaRefunded(refunded) => actor_state
                .alpha_ledger_mut()
                .transition_to_refunded(refunded),
            SwapEvent::BetaDeployed(deployed) => actor_state
                .beta_ledger_mut()
                .transition_to_deployed(deployed),
            SwapEvent::BetaFunded(funded) => {
                let expected_asset = actor_state.expected_beta_asset();

                match expected_asset.cmp(&funded.asset) {
                    Ordering::Equal => actor_state.beta_ledger_mut().transition_to_funded(funded),
                    _ => actor_state
                        .beta_ledger_mut()
                        .transition_to_incorrectly_funded(funded),
                }
            }
            SwapEvent::BetaRedeemed(redeemed) => {
                actor_state
                    .beta_ledger_mut()
                    .transition_to_redeemed(redeemed);
            }
            SwapEvent::BetaRefunded(refunded) => actor_state
                .beta_ledger_mut()
                .transition_to_refunded(refunded),
        }

        self.insert(key.clone(), actor_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset,
        asset::ethereum::FromWei,
        ethereum::Address,
        seed::{DeriveSwapSeed, RootSeed},
        swap_protocols::{
            ledger::{bitcoin, Ethereum},
            rfc003::{alice, messages::Request, Accept, Secret},
            HashFunction,
        },
        timestamp::Timestamp,
    };
    use spectral::prelude::*;

    #[test]
    fn insert_and_get_state() {
        let state_store = InMemoryStateStore::default();

        let bitcoin_pub_key = "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275"
            .parse()
            .unwrap();
        let ethereum_address: Address = "8457037fcd80a8650c4692d7fcfc1d0a96b92867".parse().unwrap();

        let request = Request {
            swap_id: SwapId::default(),
            alpha_ledger: bitcoin::Regtest {},
            beta_ledger: Ethereum::default(),
            alpha_asset: asset::Bitcoin::from_sat(100_000_000),
            beta_asset: asset::Ether::from_wei(10_000_000_000_000_000_000u64),
            hash_function: HashFunction::Sha256,
            alpha_ledger_refund_identity: bitcoin_pub_key,
            beta_ledger_redeem_identity: ethereum_address,
            alpha_expiry: Timestamp::from(2_000_000_000),
            beta_expiry: Timestamp::from(2_000_000_000),
            secret_hash: Secret::from(*b"hello world, you are beautiful!!").hash(),
        };
        let accept = Accept {
            swap_id: SwapId::default(),
            beta_ledger_refund_identity: ethereum_address,
            alpha_ledger_redeem_identity: bitcoin_pub_key,
        };

        let id = SwapId::default();
        let seed = RootSeed::from(*b"hello world, you are beautiful!!");
        let secret_source = seed.derive_swap_seed(id);
        let state = alice::State::accepted(request, accept, secret_source);

        state_store
            .insert::<alice::State<bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Ether>>(
                id,
                state.clone(),
            );

        let res = state_store
            .get::<alice::State<bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Ether>>(&id)
            .unwrap();
        assert_that(&res).contains_value(state);
    }
}

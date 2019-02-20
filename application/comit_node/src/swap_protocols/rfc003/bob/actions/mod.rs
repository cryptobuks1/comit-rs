mod btc_erc20;
mod btc_eth;
mod erc20_btc;
mod eth_btc;

use crate::{
    comit_client::{SwapDeclineReason, SwapReject},
    swap_protocols::rfc003::{
        bob::ResponseSender, messages::ToAcceptResponseBody, secret_source::SecretSource, Ledger,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund> {
    Accept(Accept),
    Decline(Decline),
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund>
    ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
{
    pub fn name(&self) -> String {
        use self::ActionKind::*;
        match *self {
            Accept(_) => String::from("accept"),
            Decline(_) => String::from("decline"),
            Deploy(_) => String::from("deploy"),
            Fund(_) => String::from("fund"),
            Redeem(_) => String::from("redeem"),
            Refund(_) => String::from("refund"),
        }
    }
}

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct Accept<AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    sender: ResponseSender<AL, BL>,
    secret_source: Arc<dyn SecretSource>,
}

impl<AL: Ledger, BL: Ledger> Accept<AL, BL> {
    #[allow(clippy::type_complexity)]
    pub fn new(sender: ResponseSender<AL, BL>, secret_source: Arc<dyn SecretSource>) -> Self {
        Self {
            sender,
            secret_source,
        }
    }
    pub fn accept<P: ToAcceptResponseBody<AL, BL>>(&self, partial_response: P) -> Result<(), ()> {
        let mut sender = self.sender.lock().unwrap();

        match sender.take() {
            Some(sender) => {
                sender
                    .send(Ok(
                        partial_response.to_accept_response_body(self.secret_source.as_ref())
                    ))
                    .expect("Action shouldn't outlive BobToAlice");
                Ok(())
            }
            None => Err(()),
        }
    }
}

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct Decline<AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    sender: ResponseSender<AL, BL>,
}

impl<AL: Ledger, BL: Ledger> Decline<AL, BL> {
    #[allow(clippy::type_complexity)]
    pub fn new(sender: ResponseSender<AL, BL>) -> Self {
        Self { sender }
    }
    pub fn decline(&self, reason: Option<SwapDeclineReason>) -> Result<(), ()> {
        let mut sender = self.sender.lock().unwrap();
        match sender.take() {
            Some(sender) => {
                sender
                    .send(Err(SwapReject::Declined { reason }))
                    .expect("Action shouldn't outlive BobToAlice");
                Ok(())
            }
            None => Err(()),
        }
    }
}

use crate::{
    http_api::action::ListRequiredFields,
    swap_protocols::rfc003::{actions::Decline, messages::SwapDeclineReason},
};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct DeclineBody {
    pub reason: Option<HttpApiSwapDeclineReason>,
}

impl<AL, BL> ListRequiredFields for Decline<AL, BL> {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![siren::Field {
            name: "reason".to_owned(),
            class: vec![],
            _type: Some("text".to_owned()),
            value: None,
            title: None,
        }]
    }
}

pub fn to_swap_decline_reason(
    reason: Option<HttpApiSwapDeclineReason>,
) -> Option<SwapDeclineReason> {
    reason.map(|reason| match reason {
        HttpApiSwapDeclineReason::UnsatisfactoryRate => SwapDeclineReason::UnsatisfactoryRate,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum HttpApiSwapDeclineReason {
    UnsatisfactoryRate,
}

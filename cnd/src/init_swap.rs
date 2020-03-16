use crate::{
    db::AcceptedSwap,
    swap_protocols::{
        ledger_states::{AlphaLedgerState, BetaLedgerState},
        rfc003::{
            create_swap::{create_watcher, OngoingSwap},
            events::{HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded},
            Accept, Request, SwapCommunication,
        },
        state, InsertFailedSwap,
    },
};
use std::sync::Arc;
use tracing_futures::Instrument;

#[allow(clippy::cognitive_complexity)]
pub fn init_accepted_swap<D, AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>(
    dependencies: &D,
    accepted: AcceptedSwap<AL, BL, AA, BA, AI, BI>,
) -> anyhow::Result<()>
where
    D: state::Insert<SwapCommunication<AL, BL, AA, BA, AI, BI>>
        + InsertFailedSwap
        + Clone
        + HtlcFunded<AL, AA, AH, AI, AT>
        + HtlcFunded<BL, BA, BH, BI, BT>
        + HtlcDeployed<AL, AA, AH, AI, AT>
        + HtlcDeployed<BL, BA, BH, BI, BT>
        + HtlcRedeemed<AL, AA, AH, AI, AT>
        + HtlcRedeemed<BL, BA, BH, BI, BT>
        + HtlcRefunded<AL, AA, AH, AI, AT>
        + HtlcRefunded<BL, BA, BH, BI, BT>,
    Arc<AlphaLedgerState>: From<D>,
    Arc<BetaLedgerState>: From<D>,
    AL: Clone + Send + Sync + 'static,
    BL: Clone + Send + Sync + 'static,
    AA: Ord + Clone + Send + Sync + 'static,
    BA: Ord + Clone + Send + Sync + 'static,
    AH: Clone + Send + Sync + 'static,
    BH: Clone + Send + Sync + 'static,
    AI: Clone + Send + Sync + 'static,
    BI: Clone + Send + Sync + 'static,
    AT: Clone + Send + Sync + 'static,
    BT: Clone + Send + Sync + 'static,
    Request<AL, BL, AA, BA, AI, BI>: Clone,
    Accept<AI, BI>: Copy,
{
    let (request, accept, accepted_at) = accepted;
    let id = request.swap_id;

    dependencies.insert(id, SwapCommunication::Accepted {
        request: request.clone(),
        response: accept,
    });

    let (alpha_htlc_params, beta_htlc_params) = {
        let swap = OngoingSwap::new(request, accept);
        (swap.alpha_htlc_params(), swap.beta_htlc_params())
    };

    tracing::trace!("initialising accepted swap: {}", id);

    tokio::task::spawn(
        create_watcher::<_, _, _, _, AH, _, AT>(
            dependencies.clone(),
            Arc::<AlphaLedgerState>::from(dependencies.clone()),
            id,
            alpha_htlc_params,
            accepted_at,
        )
        .instrument(tracing::info_span!("alpha")),
    );

    tokio::task::spawn(
        create_watcher::<_, _, _, _, BH, _, BT>(
            dependencies.clone(),
            Arc::<BetaLedgerState>::from(dependencies.clone()),
            id,
            beta_htlc_params,
            accepted_at,
        )
        .instrument(tracing::info_span!("beta")),
    );

    Ok(())
}

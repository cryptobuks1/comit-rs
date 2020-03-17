use crate::{
    handler::AnnounceHandler, AnnounceEvent, AnnounceMessage, AnnounceResult, SwapDigest, SwapId,
    PING_SIZE,
};
use futures::{future::BoxFuture, prelude::*};
use libp2p::PeerId;
use libp2p_core::{ConnectedPoint, InboundUpgrade, Multiaddr, OutboundUpgrade, UpgradeInfo};
use libp2p_swarm::{NetworkBehaviour, NetworkBehaviourAction, PollParameters};
use rand::{distributions, prelude::*};
use std::{
    collections::VecDeque,
    io, iter,
    task::{Context, Poll},
    time::Duration,
};
use void::Void;
use wasm_timer::Instant;

pub struct Announce {
    events: VecDeque<AnnounceEvent>,
}

impl Announce {
    pub fn new() -> Self {
        Announce {
            events: VecDeque::new(),
        }
    }
}

impl NetworkBehaviour for Announce {
    type ProtocolsHandler = AnnounceHandler;
    type OutEvent = AnnounceEvent;

    /// create a new protocols handler for each incoming connection or each time
    /// we dial a node
    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        AnnounceHandler::new()
    }

    /// returns a list of addresses ordered by reachability
    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _peer_id: PeerId, _endpoint: ConnectedPoint) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _endpoint: ConnectedPoint) {}

    /// Pushes protocol handler events to the network behaviour
    fn inject_node_event(&mut self, peer: PeerId, result: AnnounceMessage) {
        self.events.push_front(AnnounceEvent { peer, result })
    }

    /// handle/consume? events
    fn poll(
        &mut self,
        _: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Void, AnnounceEvent>> {
        if let Some(e) = self.events.pop_back() {
            // Poll::Ready(NetworkBehaviourAction::DialPeer {})
            match e.result {
                AnnounceMessage::Id(..) => {
                    // Close connection
                    Poll::Pending
                }
                AnnounceMessage::Digest(digest) => {
                    // Find swap id and pass it back as an event
                    // Poll::SendEvent{..}
                    Poll::Pending
                }
            }
        } else {
            Poll::Pending
        }
    }
}

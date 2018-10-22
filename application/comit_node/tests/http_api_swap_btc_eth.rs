extern crate bitcoin_support;
extern crate ethereum_support;
extern crate event_store;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_rpc_client;
extern crate comit_node;
extern crate hex;
extern crate pretty_env_logger;
extern crate reqwest;
#[macro_use]
extern crate serde_json;
extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate testcontainers;
extern crate uuid;

use comit_node::{
    comit_client::{fake::FakeClientFactory, SwapReject},
    gotham_factory::create_gotham_router,
    key_store::KeyStore,
    swap_protocols::{
        ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
        rfc003::{self, ethereum::Seconds},
    },
    swaps::common::TradeId,
};
use event_store::InMemoryEventStore;
use futures::sync::mpsc::{self, UnboundedReceiver};
use gotham::test::TestServer;
use hex::FromHex;
use hyper::{header, StatusCode};
use std::{net::SocketAddr, str::FromStr, sync::Arc};

fn build_test_server() -> (
    TestServer,
    Arc<FakeClientFactory>,
    UnboundedReceiver<TradeId>,
) {
    let _ = pretty_env_logger::try_init();
    let event_store = Arc::new(InMemoryEventStore::default());
    let fake_factory = Arc::new(FakeClientFactory::default());
    let master_priv_key =
        "xprv9s21ZrQH143K457pTbhs1LcmMnc4pCyqNTe9iEyoR8iTZeLtRzL6SpWCzK5iEP7fk72VhqkiNHuKQfqRVHTHBHQjxDDU7kTKHUuQCLNCbYi"
        .parse().unwrap();

    let (sender, receiver) = mpsc::unbounded();

    let key_store = KeyStore::new(master_priv_key).unwrap();
    let router = create_gotham_router(
        event_store,
        fake_factory.clone(),
        SocketAddr::from_str("127.0.0.1:4242").unwrap(),
        Arc::new(key_store),
        sender,
    );
    (TestServer::new(router).unwrap(), fake_factory, receiver)
}

#[test]
fn get_non_existent_swap() {
    let (test_server, _, _receiver) = build_test_server();
    let id = TradeId::default();

    let response = test_server
        .client()
        .get(format!("http://localhost/swaps/{}", id).as_str())
        .perform()
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND)
}

#[derive(Deserialize, Debug)]
struct SwapCreated {
    pub id: TradeId,
}

#[test]
fn swap_accepted_btc_eth() {
    let (test_server, fake_factory, _receiver) = build_test_server();

    let response = test_server
        .client()
        .post(
            "http://localhost/swaps",
            json!(
            {
                "source_ledger"  : {
                    "value" : "Bitcoin",
                    "identity" : "ac2db2f2615c81b83fe9366450799b4992931575",
                },
                "target_ledger" : {
                    "value" : "Ethereum",
                    "identity" : "0x00a329c0648769a73afac7f9381e08fb43dbea72"
                },
                "source_asset" : {
                    "value" : "Bitcoin",
                    "quantity" : "100000000"
                },
                "target_asset" : {
                    "value" : "Ether",
                    "quantity" : "1000000000000000000"
                }
            }
        ).to_string(),
            mime::APPLICATION_JSON,
        ).perform()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let location = {
        let headers = response.headers();
        assert!(headers.get(header::CONTENT_TYPE).is_some());
        let content_type = headers.get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, mime::APPLICATION_JSON.as_ref());

        let location = headers.get(header::LOCATION);
        assert!(location.is_some());
        location.unwrap().to_str().unwrap().to_string()
    };

    let swap_created =
        serde_json::from_slice::<SwapCreated>(response.read_body().unwrap().as_ref());

    assert!(swap_created.is_ok());

    let swap_created = swap_created.unwrap();

    assert_eq!(format!("/swaps/{}", &swap_created.id), location);

    {
        #[derive(Deserialize)]
        struct SwapPending {
            pub status: String,
        }
        let response = test_server
            .client()
            .get(format!("http://localhost/{}", location).as_str())
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let get_swap =
            serde_json::from_slice::<SwapPending>(response.read_body().unwrap().as_ref()).unwrap();

        assert_eq!(get_swap.status, "pending");
    }

    // Simulate the response
    fake_factory
        .fake_client
        .resolve_request(Ok(rfc003::AcceptResponse::<Bitcoin, Ethereum> {
            target_ledger_refund_identity: ethereum_support::Address::from_str(
                "b3474ca43d419fc54110f7dbc4626f1a2f86b4ab",
            ).unwrap(),
            source_ledger_success_identity: bitcoin_support::PubkeyHash::from_hex(
                "2107b76566056263e6f281f3a991b6651284bc76",
            ).unwrap(),
            target_ledger_lock_duration: Seconds(60 * 60 * 24),
        }));

    {
        #[derive(Deserialize)]
        struct SwapAccepted {
            pub status: String,
            pub funding_required: bitcoin_support::Address,
        }

        let response = test_server
            .client()
            .get(format!("http://localhost/swaps/{}", swap_created.id).as_str())
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let swap_accepted =
            serde_json::from_slice::<SwapAccepted>(response.read_body().unwrap().as_ref()).unwrap();

        assert_eq!(swap_accepted.status, "accepted");
    }
}

#[test]
fn swap_rejected_btc_eth() {
    let (test_server, fake_factory, _receiver) = build_test_server();

    let response = test_server
        .client()
        .post(
            "http://localhost/swaps",
            json!(
                {
                    "source_ledger"  : {
                        "value" : "Bitcoin",
                        "identity" : "ac2db2f2615c81b83fe9366450799b4992931575",
                    },
                    "target_ledger" : {
                        "value" : "Ethereum",
                        "identity" : "0x00a329c0648769a73afac7f9381e08fb43dbea72"
                    },
                    "source_asset" : {
                        "value" : "Bitcoin",
                        "quantity" : "100000000"
                    },
                    "target_asset" : {
                        "value" : "Ether",
                        "quantity" : "1000000000000000000"
                    }
                }
            ).to_string(),
            mime::APPLICATION_JSON,
        ).perform()
        .unwrap();

    let swap_created =
        serde_json::from_slice::<SwapCreated>(response.read_body().unwrap().as_ref()).unwrap();

    fake_factory
        .fake_client
        .resolve_request::<Bitcoin, Ethereum>(Err(SwapReject::Rejected));

    {
        let response = test_server
            .client()
            .get(format!("http://localhost/swaps/{}", swap_created.id).as_str())
            .perform()
            .unwrap();

        #[derive(Deserialize)]
        struct SwapRejected {
            pub status: String,
        }

        assert_eq!(response.status(), StatusCode::OK);

        let swap_rejected =
            serde_json::from_slice::<SwapRejected>(response.read_body().unwrap().as_ref()).unwrap();

        assert_eq!(swap_rejected.status, "rejected");
    }
}

import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { Action, SwapResponse } from "../../../lib/comit";
import { Wallet } from "../../../lib/wallet";
import { BN, toWei, toBN } from "web3-utils";
import { HarnessGlobal } from "../../../lib/util";

import chaiHttp = require("chai-http");

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

const tobyWallet = new Wallet("toby", {
    ethConfig: global.ledgers_config.ethereum,
});

const toby_initial_eth = "10";
const alice_initial_eth = "5";
const alice_initial_erc20 = toWei("10000", "ether");

const alice = new Actor("alice", global.config, global.test_root, {
    ethConfig: global.ledgers_config.ethereum,
    btcConfig: global.ledgers_config.bitcoin,
});
const bob = new Actor("bob", global.config, global.test_root, {
    ethConfig: global.ledgers_config.ethereum,
    btcConfig: global.ledgers_config.bitcoin,
});

const alice_final_address =
    "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
const bob_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const bob_comit_node_address = bob.comitNodeConfig.comit.comit_listen;
const alpha_asset_quantity = toBN(toWei("5000", "ether"));

const beta_asset_quantity = 100000000;
const beta_max_fee = 5000; // Max 5000 satoshis fee
const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

describe("RFC003: ERC20 for Bitcoin", () => {
    let token_contract_address: string;
    before(async function() {
        this.timeout(5000);
        await bitcoin.ensureSegwit();
        await tobyWallet.eth().fund(toby_initial_eth);
        await alice.wallet.eth().fund(alice_initial_eth);
        await bob.wallet.btc().fund(10);
        await bob.wallet.eth().fund("1");
        let receipt = await tobyWallet
            .eth()
            .deploy_erc20_token_contract(global.project_root);
        token_contract_address = receipt.contractAddress;

        await bitcoin.generate();
    });

    it(alice_initial_erc20 + " tokens were minted to Alice", async function() {
        let alice_wallet_address = alice.wallet.eth().address();

        let receipt = await ethereum.mintErc20Tokens(
            tobyWallet.eth(),
            token_contract_address,
            alice_wallet_address,
            alice_initial_erc20
        );

        receipt.status.should.equal(true);

        let erc20_balance = await ethereum.erc20Balance(
            alice_wallet_address,
            token_contract_address
        );
        erc20_balance.toString().should.equal(alice_initial_erc20);
    });

    let swap_location: string;
    let alice_swap_href: string;

    it("[Alice] Should be able to make a swap request via HTTP api", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: "Ethereum",
                    network: "regtest",
                },
                beta_ledger: {
                    name: "Bitcoin",
                    network: "regtest",
                },
                alpha_asset: {
                    name: "ERC20",
                    quantity: alpha_asset_quantity.toString(),
                    token_contract: token_contract_address,
                },
                beta_asset: {
                    name: "Bitcoin",
                    quantity: beta_asset_quantity.toString(),
                },
                alpha_ledger_refund_identity: bob_final_address,
                alpha_expiry: alpha_expiry,
                beta_expiry: beta_expiry,
                peer: bob_comit_node_address,
            });

        res.should.have.status(201);
        swap_location = res.header.location;
        swap_location.should.be.a("string");
        alice_swap_href = swap_location;
    });

    it("[Alice] Should be in IN_PROGRESS and SENT after sending the swap request to Bob", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            alice_swap_href,
            body =>
                body.status === "IN_PROGRESS" &&
                body.state.communication.status === "SENT"
        );
    });

    let bob_swap_href: string;

    it("[Bob] Shows the Swap as IN_PROGRESS in /swaps", async () => {
        let body = (await bob.poll_comit_node_until(
            "/swaps",
            body => body._embedded.swaps.length > 0
        )) as SwapResponse;

        let swap_embedded = body._embedded.swaps[0];
        swap_embedded.protocol.should.equal("rfc003");
        swap_embedded.status.should.equal("IN_PROGRESS");
        let swap_link = swap_embedded._links;
        swap_link.should.be.a("object");
        bob_swap_href = swap_link.self.href;
        bob_swap_href.should.be.a("string");
    });

    let bob_accept_href: string;

    it("[Bob] Can get the accept action after Alice sends the swap request", async function() {
        this.timeout(10000);
        let body = (await bob.poll_comit_node_until(
            bob_swap_href,
            body => body._links.accept && body._links.decline
        )) as SwapResponse;
        bob_accept_href = body._links.accept.href;
    });

    it("[Bob] Can execute the accept action", async () => {
        let bob_response = {
            beta_ledger_refund_identity: bob.wallet.eth().address(),
            alpha_ledger_redeem_identity: bob_final_address,
        };

        let accept_res = await chai
            .request(bob.comit_node_url())
            .post(bob_accept_href)
            .send(bob_response);

        accept_res.should.have.status(200);
    });

    let alice_deploy_action: Action;

    it("[Alice] Can get the fund action after Bob accepts", async function() {
        this.timeout(10000);
        let body = (await alice.poll_comit_node_until(
            alice_swap_href,
            body => body._links.deploy
        )) as SwapResponse;
        let alice_deploy_href = body._links.deploy.href;
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_deploy_href);
        res.should.have.status(200);
        alice_deploy_action = res.body;
    });

    it("[Alice] Can execute the deploy action", async () => {
        alice_deploy_action.payload.should.include.all.keys(
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        alice_deploy_action.payload.amount.should.equal("0");
        await alice.do(alice_deploy_action);
    });

    let alice_fund_action: Action;

    it("[Alice] Can get the fund action after she deploys", async function() {
        this.timeout(10000);
        let body = (await alice.poll_comit_node_until(
            alice_swap_href,
            body => body._links.fund
        )) as SwapResponse;
        let alice_fund_href = body._links.fund.href;
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_fund_href);
        res.should.have.status(200);
        alice_fund_action = res.body;
    });

    it("[Alice] Can execute the fund action", async () => {
        alice_fund_action.payload.should.include.all.keys(
            "contract_address",
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        let receipt = await alice.do(alice_fund_action);
        receipt.status.should.equal(true);
    });

    let bob_fund_action: Action;

    it("[Bob] Can get the fund action after Alice funds", async function() {
        this.timeout(10000);
        let body = (await bob.poll_comit_node_until(
            bob_swap_href,
            body => body._links.fund
        )) as SwapResponse;
        let bob_fund_href = body._links.fund.href;
        let res = await chai.request(bob.comit_node_url()).get(bob_fund_href);
        res.should.have.status(200);
        bob_fund_action = res.body;
    });

    it("[Bob] Can execute the fund action", async () => {
        bob_fund_action.payload.should.include.all.keys(
            "to",
            "amount",
            "network"
        );
        await bob.do(bob_fund_action);
        await bitcoin.generate();
    });

    let alice_redeem_action: Action;

    it("[Alice] Can get the redeem action after Bob funds", async function() {
        this.timeout(10000);
        let body = (await alice.poll_comit_node_until(
            alice_swap_href,
            body => body._links.redeem
        )) as SwapResponse;
        let alice_redeem_href = body._links.redeem.href;
        let res = await chai
            .request(alice.comit_node_url())
            .get(
                alice_redeem_href +
                    "?address=" +
                    alice_final_address +
                    "&fee_per_byte=20"
            );
        res.should.have.status(200);
        alice_redeem_action = res.body;
    });

    it("[Alice] Can execute the redeem action", async function() {
        alice_redeem_action.payload.should.include.all.keys("hex", "network");
        await alice.do(alice_redeem_action);
        await bitcoin.generate();
    });

    it("[Alice] Should have received the beta asset after the redeem", async function() {
        this.timeout(10000);
        let body = (await alice.poll_comit_node_until(
            alice_swap_href,
            body => body.state.beta_ledger.status === "Redeemed"
        )) as SwapResponse;
        let alice_redeem_txid = body.state.beta_ledger.redeem_tx;

        let alice_satoshi_received = await bitcoin.getFirstUtxoValueTransferredTo(
            alice_redeem_txid,
            alice_final_address
        );
        const alice_satoshi_expected = beta_asset_quantity - beta_max_fee;

        alice_satoshi_received.should.be.at.least(alice_satoshi_expected);
    });

    let bob_redeem_action: Action;

    it("[Bob] Can get the redeem action after Alice redeems", async function() {
        this.timeout(10000);
        let body = (await bob.poll_comit_node_until(
            bob_swap_href,
            body => body._links.redeem
        )) as SwapResponse;
        let bob_redeem_href = body._links.redeem.href;
        let res = await chai.request(bob.comit_node_url()).get(bob_redeem_href);
        res.should.have.status(200);
        bob_redeem_action = res.body;
    });

    let bob_erc20_balance_before: BN;

    it("[Bob] Can execute the redeem action", async function() {
        bob_redeem_action.payload.should.include.all.keys(
            "contract_address",
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        bob_erc20_balance_before = await ethereum.erc20Balance(
            bob_final_address,
            token_contract_address
        );
        await bob.do(bob_redeem_action);
    });

    it("[Bob] Should have received the alpha asset after the redeem", async function() {
        let bob_erc20_balance_after = await ethereum.erc20Balance(
            bob_final_address,
            token_contract_address
        );

        let bob_erc20_balance_expected = bob_erc20_balance_before.add(
            alpha_asset_quantity
        );

        bob_erc20_balance_after
            .eq(bob_erc20_balance_expected)
            .should.be.equal(true);
    });
});
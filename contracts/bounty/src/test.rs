#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn setup() -> (
    Env,
    BountyContractClient<'static>,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(BountyContract, ());
    let bounty_client = BountyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(admin);
    let token_client = token::Client::new(&env, &token_contract.address());
    let token_admin = token::StellarAssetClient::new(&env, &token_contract.address());
    token_admin.mint(&maintainer, &1_000);

    (
        env,
        bounty_client,
        token_client,
        token_admin,
        maintainer,
        contract_id,
    )
}

#[test]
fn create_bounty_locks_tokens_until_deadline() {
    let (env, bounty_client, token_client, _token_admin, maintainer, contract_id) = setup();
    let issue_id = symbol_short!("ISSUE3");

    bounty_client.create_bounty(&token_client.address, &maintainer, &issue_id, &250, &1_500);

    assert_eq!(token_client.balance(&maintainer), 750);
    assert_eq!(token_client.balance(&contract_id), 250);

    env.ledger().set_timestamp(1_500);
    bounty_client.cancel_bounty(&issue_id, &maintainer);

    assert_eq!(token_client.balance(&maintainer), 1_000);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
#[should_panic(expected = "Bounty deadline must be in the future")]
fn create_bounty_rejects_past_deadline() {
    let (_env, bounty_client, token_client, _token_admin, maintainer, _contract_id) = setup();

    bounty_client.create_bounty(
        &token_client.address,
        &maintainer,
        &symbol_short!("ISSUE3"),
        &250,
        &1_000,
    );
}

#[test]
#[should_panic(expected = "Bounty deadline has not passed")]
fn cancel_bounty_rejects_early_refund() {
    let (_env, bounty_client, token_client, _token_admin, maintainer, _contract_id) = setup();
    let issue_id = symbol_short!("ISSUE3");

    bounty_client.create_bounty(&token_client.address, &maintainer, &issue_id, &250, &1_500);
    bounty_client.cancel_bounty(&issue_id, &maintainer);
}

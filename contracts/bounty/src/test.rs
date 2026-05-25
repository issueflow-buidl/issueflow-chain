#![cfg(test)]

use super::{BountyContract, BountyContractClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{symbol_short, token, Address, Env};

#[test]
fn tip_transfers_tokens_from_sender_to_receiver() {
    let env = Env::default();
    env.mock_all_auths();

    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token = token::Client::new(&env, &token_address.address());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address.address());

    token_admin_client.mint(&sender, &1_000);

    let contract_id = env.register(BountyContract, ());
    let client = BountyContractClient::new(&env, &contract_id);

    client.tip(&token_address.address(), &sender, &receiver, &250);

    assert_eq!(token.balance(&sender), 750);
    assert_eq!(token.balance(&receiver), 250);
}

#[test]
fn create_bounty_locks_maintainer_funds_in_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token = token::Client::new(&env, &token_address.address());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address.address());

    token_admin_client.mint(&maintainer, &500);

    let contract_id = env.register(BountyContract, ());
    let client = BountyContractClient::new(&env, &contract_id);

    client.create_bounty(&token_address.address(), &maintainer, &symbol_short!("iss1"), &300);

    assert_eq!(token.balance(&maintainer), 200);
    assert_eq!(token.balance(&contract_id), 300);
}

#[test]
#[should_panic(expected = "Bounty already exists for this issue and maintainer")]
fn create_bounty_rejects_duplicate_issue_for_same_maintainer() {
    let env = Env::default();
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address.address());

    token_admin_client.mint(&maintainer, &1_000);

    let contract_id = env.register(BountyContract, ());
    let client = BountyContractClient::new(&env, &contract_id);

    let issue_id = symbol_short!("iss2");
    client.create_bounty(&token_address.address(), &maintainer, &issue_id, &100);
    client.create_bounty(&token_address.address(), &maintainer, &issue_id, &150);
}

#[test]
fn cancel_bounty_returns_funds_to_maintainer() {
    let env = Env::default();
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token = token::Client::new(&env, &token_address.address());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address.address());

    token_admin_client.mint(&maintainer, &900);

    let contract_id = env.register(BountyContract, ());
    let client = BountyContractClient::new(&env, &contract_id);

    client.create_bounty(&token_address.address(), &maintainer, &symbol_short!("iss3"), &400);
    client.cancel_bounty(&token_address.address(), &maintainer, &400);

    assert_eq!(token.balance(&contract_id), 0);
    assert_eq!(token.balance(&maintainer), 900);
}

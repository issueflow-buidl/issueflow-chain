#![cfg(test)]

use super::{BountyContract, BountyContractClient};
use soroban_sdk::{symbol_short, token, Address, Env};
use soroban_sdk::testutils::Address as _;

#[test]
fn claim_bounty_releases_locked_balance_and_clears_bounty() {
    let env = Env::default();
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token = token::Client::new(&env, &token_address.address());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address.address());

    token_admin_client.mint(&maintainer, &500);

    let contract_id = env.register(BountyContract, ());
    let client = BountyContractClient::new(&env, &contract_id);

    let issue_id = symbol_short!("iss1");
    client.create_bounty(&token_address.address(), &maintainer, &issue_id, &300);

    assert_eq!(token.balance(&maintainer), 700);
    assert_eq!(token.balance(&contract_id), 300);

    client.claim_bounty(&token_address.address(), &maintainer, &contributor, &issue_id);

    assert_eq!(token.balance(&contract_id), 0);
    assert_eq!(token.balance(&contributor), 300);
}

#[test]
#[should_panic(expected = "No bounty exists for this issue and maintainer")]
fn claim_bounty_cannot_be_executed_twice() {
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

use super::{BountyContract, BountyContractClient};
use soroban_sdk::{symbol_short, testutils::Address as _, token, Address, Env};

struct TestFixture {
    env: Env,
    client: BountyContractClient<'static>,
    token: Address,
    maintainer: Address,
    contributor: Address,
    contract: Address,
}

fn setup() -> TestFixture {
    let env = Env::default();
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
    token_admin_client.mint(&maintainer, &1_000);

    let contract = env.register(BountyContract, ());
    let client = BountyContractClient::new(&env, &contract);

    TestFixture {
        env,
        client,
        token: token_address,
        maintainer,
        contributor,
        contract,
    }
}

#[test]
fn create_bounty_locks_tokens_in_contract_escrow() {
    let fixture = setup();
    let token_client = token::Client::new(&fixture.env, &fixture.token);
    let issue_id = symbol_short!("ISSUE2");

    fixture
        .client
        .create_bounty(&fixture.token, &fixture.maintainer, &issue_id, &250);

    assert_eq!(token_client.balance(&fixture.maintainer), 750);
    assert_eq!(token_client.balance(&fixture.contract), 250);
    assert_eq!(
        fixture
            .client
            .get_bounty_amount(&fixture.maintainer, &issue_id),
        250
    );
}

#[test]
fn claim_bounty_releases_locked_tokens_to_contributor() {
    let fixture = setup();
    let token_client = token::Client::new(&fixture.env, &fixture.token);
    let issue_id = symbol_short!("ISSUE2");

    fixture
        .client
        .create_bounty(&fixture.token, &fixture.maintainer, &issue_id, &250);
    fixture
        .client
        .claim_bounty(&fixture.maintainer, &issue_id, &fixture.contributor);

    assert_eq!(token_client.balance(&fixture.maintainer), 750);
    assert_eq!(token_client.balance(&fixture.contributor), 250);
    assert_eq!(token_client.balance(&fixture.contract), 0);
    assert_eq!(
        fixture
            .client
            .get_bounty_amount(&fixture.maintainer, &issue_id),
        0
    );
}

#[test]
fn claim_bounty_uses_the_escrowed_token() {
    let fixture = setup();
    let issue_id = symbol_short!("ISSUE2");
    let other_token_admin = Address::generate(&fixture.env);
    let other_token = fixture
        .env
        .register_stellar_asset_contract_v2(other_token_admin.clone());
    let other_token_address = other_token.address();
    let other_admin_client = token::StellarAssetClient::new(&fixture.env, &other_token_address);
    let other_token_client = token::Client::new(&fixture.env, &other_token_address);
    let escrowed_token_client = token::Client::new(&fixture.env, &fixture.token);

    other_admin_client.mint(&fixture.contract, &999);
    fixture
        .client
        .create_bounty(&fixture.token, &fixture.maintainer, &issue_id, &250);
    fixture
        .client
        .claim_bounty(&fixture.maintainer, &issue_id, &fixture.contributor);

    assert_eq!(escrowed_token_client.balance(&fixture.contributor), 250);
    assert_eq!(other_token_client.balance(&fixture.contributor), 0);
    assert_eq!(other_token_client.balance(&fixture.contract), 999);
}

#[test]
fn cancel_bounty_returns_locked_tokens_to_maintainer() {
    let fixture = setup();
    let token_client = token::Client::new(&fixture.env, &fixture.token);
    let issue_id = symbol_short!("ISSUE2");

    fixture
        .client
        .create_bounty(&fixture.token, &fixture.maintainer, &issue_id, &250);
    fixture.client.cancel_bounty(&fixture.maintainer, &issue_id);

    assert_eq!(token_client.balance(&fixture.maintainer), 1_000);
    assert_eq!(token_client.balance(&fixture.contract), 0);
    assert_eq!(
        fixture
            .client
            .get_bounty_amount(&fixture.maintainer, &issue_id),
        0
    );
}

#[test]
#[should_panic(expected = "Bounty not found")]
fn claim_bounty_rejects_missing_escrow() {
    let fixture = setup();

    fixture.client.claim_bounty(
        &fixture.maintainer,
        &symbol_short!("NONE"),
        &fixture.contributor,
    );
}

#[test]
#[should_panic(expected = "Bounty not found")]
fn claim_bounty_cannot_be_paid_twice() {
    let fixture = setup();
    let issue_id = symbol_short!("ISSUE2");

    fixture
        .client
        .create_bounty(&fixture.token, &fixture.maintainer, &issue_id, &250);
    fixture
        .client
        .claim_bounty(&fixture.maintainer, &issue_id, &fixture.contributor);
    fixture
        .client
        .claim_bounty(&fixture.maintainer, &issue_id, &fixture.contributor);
}

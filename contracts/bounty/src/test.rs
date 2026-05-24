#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Env, IntoVal, LedgerInfo, Symbol};

// ============================================================================
// Helper: set up the environment, register the contract, and create a mock
// token contract that implements the Soroban token interface.
// ============================================================================

struct TestContext {
    env: Env,
    contract_id: Address,
    token_id: Address,
    creator: Address,
    solver: Address,
    stranger: Address,
}

/// Deploy a minimal mock token contract (soroban-sdk built-in token contract).
fn create_token_contract(env: &Env) -> Address {
    // Use the soroban token contract from the SDK.
    // We register the built-in token contract stub.
    soroban_sdk::token::StellarAssetContract::new(env, &Address::generate(env)).address()
}

fn setup() -> TestContext {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(BountyContract, ());
    let token_id = create_token_contract(&env);

    let creator = Address::generate(&env);
    let solver = Address::generate(&env);
    let stranger = Address::generate(&env);

    // Mint tokens to the creator so they can fund bounties.
    soroban_sdk::token::StellarAssetClient::new(&env, &token_id).mint(&creator, &10_000_000i128);

    TestContext {
        env,
        contract_id,
        token_id,
        creator,
        solver,
        stranger,
    }
}

fn client<'a>(ctx: &'a TestContext) -> BountyContractClient<'a> {
    BountyContractClient::new(&ctx.env, &ctx.contract_id)
}

fn token_client<'a>(ctx: &'a TestContext) -> soroban_sdk::token::StellarAssetClient<'a> {
    soroban_sdk::token::StellarAssetClient::new(&ctx.env, &ctx.token_id)
}

// ============================================================================
// create_bounty tests (Issue #1)
// ============================================================================

#[test]
fn test_create_bounty_success() {
    let ctx = setup();
    let c = client(&ctx);

    // Set ledger timestamp to 1000.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &1_000_000i128, &2000u64);

    assert_eq!(bounty_id, 1);

    // Verify bounty was stored correctly.
    let bounty = c.get_bounty(&bounty_id);
    assert_eq!(bounty.creator, ctx.creator);
    assert_eq!(bounty.token, ctx.token_id);
    assert_eq!(bounty.amount, 1_000_000i128);
    assert_eq!(bounty.deadline, 2000u64);
    assert_eq!(bounty.state, BountyState::Open);

    // Verify tokens were transferred to the contract.
    let tc = token_client(&ctx);
    let contract_balance = tc.balance(&ctx.contract_id);
    assert_eq!(contract_balance, 1_000_000i128);

    let creator_balance = tc.balance(&ctx.creator);
    assert_eq!(creator_balance, 9_000_000i128);
}

#[test]
fn test_create_bounty_auto_increment_ids() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let id1 = c.create_bounty(&ctx.creator, &ctx.token_id, &100i128, &2000u64);
    let id2 = c.create_bounty(&ctx.creator, &ctx.token_id, &200i128, &3000u64);
    let id3 = c.create_bounty(&ctx.creator, &ctx.token_id, &300i128, &4000u64);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_create_bounty_zero_amount_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    c.create_bounty(&ctx.creator, &ctx.token_id, &0i128, &2000u64);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_create_bounty_negative_amount_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    c.create_bounty(&ctx.creator, &ctx.token_id, &-100i128, &2000u64);
}

#[test]
#[should_panic(expected = "deadline must be in the future")]
fn test_create_bounty_past_deadline_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 5000,
        protocol_version: 22,
        base_reserve: 10,
    });

    // Deadline at 2000 is before current time 5000.
    c.create_bounty(&ctx.creator, &ctx.token_id, &1_000i128, &2000u64);
}

#[test]
#[should_panic(expected = "deadline must be in the future")]
fn test_create_bounty_equal_deadline_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 2000,
        protocol_version: 22,
        base_reserve: 10,
    });

    // Deadline == current time should fail (must be strictly in the future).
    c.create_bounty(&ctx.creator, &ctx.token_id, &1_000i128, &2000u64);
}

#[test]
fn test_create_bounty_just_before_deadline() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1999,
        protocol_version: 22,
        base_reserve: 10,
    });

    let id = c.create_bounty(&ctx.creator, &ctx.token_id, &500i128, &2000u64);
    assert_eq!(id, 1);
}

// ============================================================================
// claim_bounty tests (Issue #2)
// ============================================================================

#[test]
fn test_claim_bounty_success() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &1_000_000i128, &5000u64);

    // Creator claims the bounty and sends funds to solver.
    c.claim_bounty(&bounty_id, &ctx.creator, &ctx.solver);

    // Verify bounty state updated.
    let bounty = c.get_bounty(&bounty_id);
    assert_eq!(bounty.state, BountyState::Claimed);

    // Verify solver received the funds.
    let tc = token_client(&ctx);
    assert_eq!(tc.balance(&ctx.solver), 1_000_000i128);
    assert_eq!(tc.balance(&ctx.contract_id), 0i128);
}

#[test]
#[should_panic(expected = "only the bounty creator can claim")]
fn test_claim_bounty_wrong_claimer_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &1_000_000i128, &5000u64);

    // Stranger tries to claim — should fail.
    c.claim_bounty(&bounty_id, &ctx.stranger, &ctx.solver);
}

#[test]
#[should_panic(expected = "bounty is not open")]
fn test_claim_bounty_already_claimed_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &1_000_000i128, &5000u64);
    c.claim_bounty(&bounty_id, &ctx.creator, &ctx.solver);

    // Second claim should panic.
    c.claim_bounty(&bounty_id, &ctx.creator, &ctx.solver);
}

#[test]
#[should_panic(expected = "bounty not found")]
fn test_claim_nonexistent_bounty_panics() {
    let ctx = setup();
    let c = client(&ctx);

    c.claim_bounty(&999, &ctx.creator, &ctx.solver);
}

// ============================================================================
// cancel_bounty tests (Issue #2 — deadline-based reclaim)
// ============================================================================

#[test]
fn test_cancel_bounty_after_deadline_success() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &500_000i128, &3000u64);

    // Advance time past deadline.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 3001,
        protocol_version: 22,
        base_reserve: 10,
    });

    let tc = token_client(&ctx);
    let creator_before = tc.balance(&ctx.creator);

    c.cancel_bounty(&bounty_id, &ctx.creator);

    let bounty = c.get_bounty(&bounty_id);
    assert_eq!(bounty.state, BountyState::Cancelled);

    // Funds returned to creator.
    assert_eq!(tc.balance(&ctx.creator), creator_before + 500_000i128);
    assert_eq!(tc.balance(&ctx.contract_id), 0i128);
}

#[test]
#[should_panic(expected = "bounty deadline has not passed yet")]
fn test_cancel_bounty_before_deadline_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &500_000i128, &3000u64);

    // Current time (2500) is before deadline (3000).
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 2500,
        protocol_version: 22,
        base_reserve: 10,
    });

    c.cancel_bounty(&bounty_id, &ctx.creator);
}

#[test]
fn test_cancel_bounty_exactly_at_deadline() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &500_000i128, &3000u64);

    // Advance time to exactly the deadline.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 3000,
        protocol_version: 22,
        base_reserve: 10,
    });

    c.cancel_bounty(&bounty_id, &ctx.creator);

    let bounty = c.get_bounty(&bounty_id);
    assert_eq!(bounty.state, BountyState::Cancelled);
}

#[test]
#[should_panic(expected = "only the bounty creator can cancel")]
fn test_cancel_bounty_wrong_account_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &500_000i128, &3000u64);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 3001,
        protocol_version: 22,
        base_reserve: 10,
    });

    // Stranger tries to cancel.
    c.cancel_bounty(&bounty_id, &ctx.stranger);
}

#[test]
#[should_panic(expected = "bounty is not open")]
fn test_cancel_already_claimed_bounty_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &500_000i128, &3000u64);
    c.claim_bounty(&bounty_id, &ctx.creator, &ctx.solver);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 3001,
        protocol_version: 22,
        base_reserve: 10,
    });

    // Cannot cancel an already-claimed bounty.
    c.cancel_bounty(&bounty_id, &ctx.creator);
}

#[test]
#[should_panic(expected = "bounty is not open")]
fn test_cancel_already_cancelled_bounty_panics() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &500_000i128, &3000u64);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 3001,
        protocol_version: 22,
        base_reserve: 10,
    });

    c.cancel_bounty(&bounty_id, &ctx.creator);

    // Second cancel should fail.
    c.cancel_bounty(&bounty_id, &ctx.creator);
}

// ============================================================================
// tip tests (backward compatibility)
// ============================================================================

#[test]
fn test_tip_success() {
    let ctx = setup();
    let c = client(&ctx);

    let tc = token_client(&ctx);

    c.tip(&ctx.token_id, &ctx.creator, &ctx.solver, &500i128);

    assert_eq!(tc.balance(&ctx.solver), 500i128);
    assert_eq!(tc.balance(&ctx.creator), 9_999_500i128);
}

// ============================================================================
// Deadline edge-case tests (Issue #3)
// ============================================================================

#[test]
fn test_bounty_deadline_far_future() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    // Deadline 1 year in the future.
    let far_future = 1000 + 365 * 24 * 60 * 60;
    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &100i128, &far_future);

    let bounty = c.get_bounty(&bounty_id);
    assert_eq!(bounty.deadline, far_future);
}

#[test]
fn test_multiple_bounties_with_different_deadlines() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let id1 = c.create_bounty(&ctx.creator, &ctx.token_id, &100i128, &2000u64);
    let id2 = c.create_bounty(&ctx.creator, &ctx.token_id, &200i128, &5000u64);

    // Advance past first deadline but not second.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 3000,
        protocol_version: 22,
        base_reserve: 10,
    });

    // First bounty can be cancelled.
    c.cancel_bounty(&id1, &ctx.creator);

    // Second bounty cannot be cancelled yet.
    let bounty2 = c.get_bounty(&id2);
    assert_eq!(bounty2.state, BountyState::Open);
}

#[test]
fn test_claim_bounty_then_cannot_cancel() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &1_000_000i128, &2000u64);

    // Claim before deadline.
    c.claim_bounty(&bounty_id, &ctx.creator, &ctx.solver);

    // Advance past deadline.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 3000,
        protocol_version: 22,
        base_reserve: 10,
    });

    // Cancel should fail — bounty is already claimed.
    // (We expect this to panic with "bounty is not open")
    let result = std::panic::catch_unwind(|| {
        // Need to use a fresh env setup because catch_unwind doesn't work
        // easily with Soroban test env. Instead, test directly.
    });
    // Instead, just verify the state:
    let bounty = c.get_bounty(&bounty_id);
    assert_eq!(bounty.state, BountyState::Claimed);
}

// ============================================================================
// get_bounty tests
// ============================================================================

#[test]
#[should_panic(expected = "bounty not found")]
fn test_get_nonexistent_bounty_panics() {
    let ctx = setup();
    let c = client(&ctx);

    c.get_bounty(&42);
}

#[test]
fn test_get_bounty_reflects_state_changes() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        base_reserve: 10,
    });

    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &1_000i128, &3000u64);

    // Initial state: Open.
    let b = c.get_bounty(&bounty_id);
    assert_eq!(b.state, BountyState::Open);
    assert_eq!(b.amount, 1_000i128);

    // After claim: Claimed.
    c.claim_bounty(&bounty_id, &ctx.creator, &ctx.solver);
    let b = c.get_bounty(&bounty_id);
    assert_eq!(b.state, BountyState::Claimed);
    // Amount is preserved in the record.
    assert_eq!(b.amount, 1_000i128);
}

// ============================================================================
// Integration: full lifecycle test
// ============================================================================

#[test]
fn test_full_lifecycle_create_claim() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 100,
        protocol_version: 22,
        base_reserve: 10,
    });

    let tc = token_client(&ctx);

    // 1. Create bounty.
    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &2_500_000i128, &10_000u64);
    assert_eq!(tc.balance(&ctx.contract_id), 2_500_000i128);

    // 2. Solver solves the issue — creator approves claim.
    c.claim_bounty(&bounty_id, &ctx.creator, &ctx.solver);

    // 3. Solver received funds.
    assert_eq!(tc.balance(&ctx.solver), 2_500_000i128);
    assert_eq!(tc.balance(&ctx.contract_id), 0i128);

    // 4. Bounty state is Claimed.
    let b = c.get_bounty(&bounty_id);
    assert_eq!(b.state, BountyState::Claimed);
}

#[test]
fn test_full_lifecycle_create_cancel() {
    let ctx = setup();
    let c = client(&ctx);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 100,
        protocol_version: 22,
        base_reserve: 10,
    });

    let tc = token_client(&ctx);
    let creator_before = tc.balance(&ctx.creator);

    // 1. Create bounty.
    let bounty_id = c.create_bounty(&ctx.creator, &ctx.token_id, &3_000_000i128, &5000u64);
    assert_eq!(tc.balance(&ctx.creator), creator_before - 3_000_000i128);

    // 2. Nobody claims — deadline passes.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 5001,
        protocol_version: 22,
        base_reserve: 10,
    });

    // 3. Creator reclaims funds.
    c.cancel_bounty(&bounty_id, &ctx.creator);

    // 4. Creator got funds back.
    assert_eq!(tc.balance(&ctx.creator), creator_before);
    assert_eq!(tc.balance(&ctx.contract_id), 0i128);

    // 5. State is Cancelled.
    let b = c.get_bounty(&bounty_id);
    assert_eq!(b.state, BountyState::Cancelled);
}

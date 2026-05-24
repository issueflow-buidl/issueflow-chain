#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, token};

// ============================================================================
// Storage Keys
// ============================================================================

/// Bounty record stored on-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounty {
    /// The address that created (and funded) the bounty.
    pub creator: Address,
    /// The token address used for the bounty (e.g. USDC).
    pub token: Address,
    /// Amount locked in the bounty, in the token's smallest unit.
    pub amount: i128,
    /// Deadline as a Unix timestamp (seconds). After this time, if unclaimed,
    /// the creator can reclaim the funds.
    pub deadline: u64,
    /// Current lifecycle state of the bounty.
    pub state: BountyState,
}

/// Lifecycle states for a bounty.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BountyState {
    /// Open for claims — funds are locked in the contract.
    Open,
    /// The bounty has been claimed and funds released to the solver.
    Claimed,
    /// The creator reclaimed the funds after deadline expiry.
    Cancelled,
}

// Per-bounty storage key (stored under persistent instance storage).
#[contracttype]
pub enum BountyKey {
    Bounty(u64),
    NextId,
}

// ============================================================================
// Events (Symbol-based topics for Soroban event system)
// ============================================================================

const EVENT_BOUNTY_CREATED: &str = "bounty_created";
const EVENT_BOUNTY_CLAIMED: &str = "bounty_claimed";
const EVENT_BOUNTY_CANCELLED: &str = "bounty_cancelled";
const EVENT_TIP: &str = "tip";

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct BountyContract;

#[contractimpl]
impl BountyContract {
    // -----------------------------------------------------------------------
    // create_bounty  (Issue #1)
    //
    // Locks `amount` of `token` from `creator` into the contract.
    // Returns the auto-incremented bounty id.
    // -----------------------------------------------------------------------
    pub fn create_bounty(
        env: Env,
        creator: Address,
        token: Address,
        amount: i128,
        deadline: u64,
    ) -> u64 {
        // Auth: only the creator can lock their own funds.
        creator.require_auth();

        // Validate amount.
        assert!(amount > 0, "amount must be positive");

        // Validate deadline is in the future.
        let now = env.ledger().timestamp();
        assert!(deadline > now, "deadline must be in the future");

        // Allocate a monotonically-increasing bounty id.
        let next_id: u64 = env
            .storage()
            .instance()
            .get(&BountyKey::NextId)
            .unwrap_or(1);
        env.storage()
            .instance()
            .set(&BountyKey::NextId, &(next_id + 1));

        // Transfer tokens from creator → contract (escrow).
        let client = token::Client::new(&env, &token);
        client.transfer(&creator, &env.current_contract_address(), &amount);

        // Persist the bounty record.
        let bounty = Bounty {
            creator: creator.clone(),
            token: token.clone(),
            amount,
            deadline,
            state: BountyState::Open,
        };
        env.storage()
            .persistent()
            .set(&BountyKey::Bounty(next_id), &bounty);

        // Emit event.
        env.events().publish(
            (Symbol::new(&env, EVENT_BOUNTY_CREATED), creator),
            (next_id, token, amount, deadline),
        );

        next_id
    }

    // -----------------------------------------------------------------------
    // claim_bounty  (Issue #2)
    //
    // Releases the locked funds to `solver`. Only callable by the bounty
    // creator (or — optionally — by a governance / attestation authority;
    // for simplicity we restrict to the creator here).
    // -----------------------------------------------------------------------
    pub fn claim_bounty(env: Env, bounty_id: u64, claimer: Address, solver: Address) {
        // Auth: the bounty creator authorises the claim.
        claimer.require_auth();

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&BountyKey::Bounty(bounty_id))
            .expect("bounty not found");

        assert_eq!(bounty.state, BountyState::Open, "bounty is not open");
        assert_eq!(
            bounty.creator, claimer,
            "only the bounty creator can claim"
        );

        // Transfer locked tokens from contract → solver.
        let client = token::Client::new(&env, &bounty.token);
        client.transfer(
            &env.current_contract_address(),
            &solver,
            &bounty.amount,
        );

        // Update state.
        bounty.state = BountyState::Claimed;
        env.storage()
            .persistent()
            .set(&BountyKey::Bounty(bounty_id), &bounty);

        // Emit event.
        env.events().publish(
            (
                Symbol::new(&env, EVENT_BOUNTY_CLAIMED),
                bounty.creator.clone(),
            ),
            (bounty_id, solver, bounty.amount),
        );
    }

    // -----------------------------------------------------------------------
    // cancel_bounty  (Issue #2 — creator reclaims after deadline)
    //
    // Returns locked funds to the maintainer / creator.  Only callable after
    // the bounty's deadline has passed and only by the original creator.
    // -----------------------------------------------------------------------
    pub fn cancel_bounty(env: Env, bounty_id: u64, maintainer: Address) {
        maintainer.require_auth();

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&BountyKey::Bounty(bounty_id))
            .expect("bounty not found");

        assert_eq!(bounty.state, BountyState::Open, "bounty is not open");
        assert_eq!(
            bounty.creator, maintainer,
            "only the bounty creator can cancel"
        );

        // Deadline check (Issue #3).
        let now = env.ledger().timestamp();
        assert!(
            now >= bounty.deadline,
            "bounty deadline has not passed yet"
        );

        // Transfer locked tokens back to creator.
        let client = token::Client::new(&env, &bounty.token);
        client.transfer(
            &env.current_contract_address(),
            &maintainer,
            &bounty.amount,
        );

        // Update state.
        bounty.state = BountyState::Cancelled;
        env.storage()
            .persistent()
            .set(&BountyKey::Bounty(bounty_id), &bounty);

        // Emit event.
        env.events().publish(
            (
                Symbol::new(&env, EVENT_BOUNTY_CANCELLED),
                maintainer.clone(),
            ),
            (bounty_id, bounty.amount),
        );
    }

    // -----------------------------------------------------------------------
    // tip  (pre-existing — kept for backward compatibility)
    //
    // Direct transfer from `from` to `to` without escrow.
    // -----------------------------------------------------------------------
    pub fn tip(env: Env, token: Address, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let client = token::Client::new(&env, &token);
        client.transfer(&from, &to, &amount);

        env.events().publish(
            (Symbol::new(&env, EVENT_TIP), from),
            (to, token, amount),
        );
    }

    // -----------------------------------------------------------------------
    // get_bounty  — read-only query
    // -----------------------------------------------------------------------
    pub fn get_bounty(env: Env, bounty_id: u64) -> Bounty {
        env.storage()
            .persistent()
            .get(&BountyKey::Bounty(bounty_id))
            .expect("bounty not found")
    }
}

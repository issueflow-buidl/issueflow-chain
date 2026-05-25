#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Map, Symbol,
};

#[contract]
pub struct BountyContract;

#[contracttype]
#[derive(Clone)]
pub struct BountyEscrow {
    pub token: Address,
    pub amount: i128,
}

#[contractimpl]
impl BountyContract {
    pub fn tip(env: Env, token: Address, from: Address, to: Address, amount: i128) {
        require_positive_amount(amount);
        from.require_auth();
        let client = token::Client::new(&env, &token);
        client.transfer(&from, &to, &amount);
    }

    pub fn create_bounty(
        env: Env,
        token: Address,
        maintainer: Address,
        issue_id: Symbol,
        amount: i128,
    ) {
        require_positive_amount(amount);
        maintainer.require_auth();

        let client = token::Client::new(&env, &token);
        client.transfer(&maintainer, env.current_contract_address(), &amount);

        let key = bounty_key(issue_id);
        let mut bounty: Map<Address, BountyEscrow> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Map::new(&env));

        if bounty.contains_key(maintainer.clone()) {
            panic!("Bounty already exists for this issue and maintainer");
        }

        bounty.set(maintainer.clone(), BountyEscrow { token, amount });
        env.storage().persistent().set(&key, &bounty);
    }

    pub fn claim_bounty(env: Env, maintainer: Address, issue_id: Symbol, contributor: Address) {
        maintainer.require_auth();

        let escrow = take_bounty(&env, maintainer, issue_id);
        let client = token::Client::new(&env, &escrow.token);
        client.transfer(&env.current_contract_address(), contributor, &escrow.amount);
    }

    pub fn get_bounty_amount(env: Env, maintainer: Address, issue_id: Symbol) -> i128 {
        let key = bounty_key(issue_id);
        let bounty: Map<Address, BountyEscrow> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Map::new(&env));

        bounty.get(maintainer).map(|b| b.amount).unwrap_or(0)
    }

    pub fn cancel_bounty(env: Env, maintainer: Address, issue_id: Symbol) {
        maintainer.require_auth();

        let escrow = take_bounty(&env, maintainer.clone(), issue_id);
        let client = token::Client::new(&env, &escrow.token);
        client.transfer(&env.current_contract_address(), maintainer, &escrow.amount);
    }
}

fn bounty_key(issue_id: Symbol) -> (Symbol, Symbol) {
    (symbol_short!("bounty"), issue_id)
}

fn require_positive_amount(amount: i128) {
    if amount <= 0 {
        panic!("Amount must be positive");
    }
}

fn take_bounty(env: &Env, maintainer: Address, issue_id: Symbol) -> BountyEscrow {
    let key = bounty_key(issue_id);
    let mut bounty: Map<Address, BountyEscrow> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Map::new(env));
    let escrow = bounty
        .get(maintainer.clone())
        .unwrap_or_else(|| panic!("Bounty not found"));

    bounty.remove(maintainer);
    if bounty.is_empty() {
        env.storage().persistent().remove(&key);
    } else {
        env.storage().persistent().set(&key, &bounty);
    }

    escrow
}

#[cfg(test)]
mod test;

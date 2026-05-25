#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Map, Symbol,
};

#[cfg(test)]
mod test;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BountyInfo {
    pub token: Address,
    pub amount: i128,
    pub deadline: u64,
}

#[contract]
pub struct BountyContract;

#[contractimpl]
impl BountyContract {
    pub fn tip(env: Env, token: Address, from: Address, to: Address, amount: i128) {
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
        deadline: u64,
    ) {
        maintainer.require_auth();
        assert!(amount > 0, "Bounty amount must be positive");
        assert!(
            deadline > env.ledger().timestamp(),
            "Bounty deadline must be in the future"
        );

        let client = token::Client::new(&env, &token);
        client.transfer(&maintainer, &env.current_contract_address(), &amount);

        let key = (symbol_short!("bounty"), issue_id.clone());
        let mut bounty: Map<Address, BountyInfo> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Map::new(&env));

        if bounty.contains_key(maintainer.clone()) {
            panic!("Bounty already exists for this issue and maintainer");
        }

        bounty.set(
            maintainer.clone(),
            BountyInfo {
                token,
                amount,
                deadline,
            },
        );
        env.storage().persistent().set(&key, &bounty);
    }

    pub fn cancel_bounty(env: Env, issue_id: Symbol, maintainer: Address) {
        maintainer.require_auth();

        let key = (symbol_short!("bounty"), issue_id.clone());
        let mut bounty: Map<Address, BountyInfo> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Map::new(&env));
        let info = bounty
            .get(maintainer.clone())
            .expect("Bounty does not exist for this issue and maintainer");

        assert!(
            env.ledger().timestamp() >= info.deadline,
            "Bounty deadline has not passed"
        );

        let client = token::Client::new(&env, &info.token);
        client.transfer(&env.current_contract_address(), &maintainer, &info.amount);

        bounty.remove(maintainer);
        env.storage().persistent().set(&key, &bounty);
    }
}

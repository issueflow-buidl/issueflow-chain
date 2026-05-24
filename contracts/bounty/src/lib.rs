#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, token, symbol_short, Symbol, Map};

#[contract]
pub struct BountyContract;

#[contractimpl]
impl BountyContract {
    pub fn tip(env: Env, token: Address, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let client = token::Client::new(&env, &token);
        client.transfer(&from, &to, &amount);
    }

    pub fn create_bounty(env: Env, token: Address, maintainer: Address, issue_id: Symbol, amount: i128) {
        maintainer.require_auth();
        
        let client = token::Client::new(&env, &token);
        client.transfer(&maintainer, &env.current_contract_address(), &amount);
        
        let key = (symbol_short!("bounty"), issue_id.clone());
        let mut bounty: Map<Address, i128> = env.storage().persistent().get(&key).unwrap_or(Map::new(&env));
        
        if bounty.contains_key(maintainer.clone()) {
            panic!("Bounty already exists for this issue and maintainer");
        }
        
        bounty.set(maintainer.clone(), amount);
        env.storage().persistent().set(&key, &bounty);
    }

    pub fn cancel_bounty(env: Env, token: Address, maintainer: Address, amount: i128) {
        maintainer.require_auth();
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &maintainer, &amount);
    }
}

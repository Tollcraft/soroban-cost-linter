#![no_std]
use soroban_sdk::symbol_short;
use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, Map, Symbol, Vec};

const RESERVE_A: Symbol = symbol_short!("reserve_a");
const RESERVE_B: Symbol = symbol_short!("reserve_b");
const LP_SHARES: Symbol = symbol_short!("lp_shares");

#[contract]
pub struct SwapContract;

#[contractimpl]
impl SwapContract {
    pub fn init(env: Env, token_a: Address, token_b: Address) {
        env.storage().instance().set(&symbol_short!("tok_a"), &token_a);
        env.storage().instance().set(&symbol_short!("tok_b"), &token_b);
        env.storage().instance().set(&RESERVE_A, &0i128);
        env.storage().instance().set(&RESERVE_B, &0i128);
    }

    pub fn add_liquidity(env: Env, from: Address, amount_a: i128, amount_b: i128) {
        from.require_auth();
        let res_a: i128 = env.storage().instance().get(&RESERVE_A).unwrap_or(0);
        let res_b: i128 = env.storage().instance().get(&RESERVE_B).unwrap_or(0);
        env.storage().instance().set(&RESERVE_A, &(res_a + amount_a));
        env.storage().instance().set(&RESERVE_B, &(res_b + amount_b));
        let shares: i128 = env.storage().instance().get(&from).unwrap_or(0);
        env.storage().instance().set(&from, &(shares + amount_a + amount_b));
    }

    pub fn swap_a_for_b(env: Env, from: Address, amount_in: i128) -> i128 {
        from.require_auth();
        let res_a: i128 = env.storage().instance().get(&RESERVE_A).unwrap_or(0);
        let res_b: i128 = env.storage().instance().get(&RESERVE_B).unwrap_or(0);
        let amount_out = (amount_in * res_b) / (res_a + amount_in);
        env.storage().instance().set(&RESERVE_A, &(res_a + amount_in));
        env.storage().instance().set(&RESERVE_B, &(res_b - amount_out));
        amount_out
    }

    pub fn swap_b_for_a(env: Env, from: Address, amount_in: i128) -> i128 {
        from.require_auth();
        let res_a: i128 = env.storage().instance().get(&RESERVE_A).unwrap_or(0);
        let res_b: i128 = env.storage().instance().get(&RESERVE_B).unwrap_or(0);
        let amount_out = (amount_in * res_a) / (res_b + amount_in);
        env.storage().instance().set(&RESERVE_A, &(res_a - amount_out));
        env.storage().instance().set(&RESERVE_B, &(res_b + amount_in));
        amount_out
    }

    pub fn batch_swap(env: Env, from: Address, amounts: Vec<i128>) -> Vec<i128> {
        from.require_auth();
        let res_a: i128 = env.storage().instance().get(&RESERVE_A).unwrap_or(0);
        let mut outcomes = Vec::new(&env);
        for amount_in in amounts.iter() {
            let res_b: i128 = env.storage().instance().get(&RESERVE_B).unwrap_or(0);
            let amount_out = (amount_in * res_b) / (res_a + amount_in);
            env.storage().instance().set(&RESERVE_A, &(res_a + amount_in));
            outcomes.push_back(amount_out);
        }
        outcomes
    }

    pub fn collect_fees(env: Env, from: Address) -> Bytes {
        from.require_auth();
        let res_a: i128 = env.storage().instance().get(&RESERVE_A).unwrap_or(0);
        let fee = res_a / 1000;
        let mut buf = Bytes::new(&env);
        let fee_bytes = Bytes::from_array(&env, &fee.to_be_bytes());
        buf.append(&fee_bytes);
        for _ in 0..3 {
            buf.append(&Bytes::from_array(&env, &[0u8; 4]));
        }
        buf
    }
}

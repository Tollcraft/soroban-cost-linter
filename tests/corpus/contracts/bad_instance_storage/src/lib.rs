#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Vec, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn push(env: Env, val: Symbol) {
        let mut vec: Vec<Symbol> = env.storage().instance().get(&Symbol::new(&env, "data")).unwrap_or(Vec::new(&env));
        vec.push_back(val);
        env.storage().instance().set(&Symbol::new(&env, "data"), &vec);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_push() {
        let env = Env::default();
        let contract_id = env.register_contract(None, Contract);
        let client = ContractClient::new(&env, &contract_id);

        let val1 = Symbol::new(&env, "test1");
        client.push(&val1);

        // Verify we can push again
        let val2 = Symbol::new(&env, "test2");
        client.push(&val2);
    }
}

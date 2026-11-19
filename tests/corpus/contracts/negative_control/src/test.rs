use super::*;
use soroban_sdk::Env;

#[test]
fn test_process_data() {
    let env = Env::default();
    let contract_id = env.register_contract(None, NegativeControlContract);
    let client = NegativeControlContractClient::new(&env, &contract_id);

    let count = 5;
    // initial state is 0. 0 + 0 + 1 + 2 + 3 + 4 = 10
    let res = client.process_data(&count);
    assert_eq!(res, 10);

    // second call, state is 10. 10 + 0 + 1 + 2 + 3 + 4 = 20
    let res2 = client.process_data(&count);
    assert_eq!(res2, 20);
}

#[test]
fn test_process_data_max_limit() {
    let env = Env::default();
    let contract_id = env.register_contract(None, NegativeControlContract);
    let client = NegativeControlContractClient::new(&env, &contract_id);

    let count = 150;
    // limit is 100. Sum of 0..99 is 4950
    let res = client.process_data(&count);
    assert_eq!(res, 4950);
}

#[test]
fn test_lookup_key() {
    let env = Env::default();
    let contract_id = env.register_contract(None, NegativeControlContract);
    let client = NegativeControlContractClient::new(&env, &contract_id);

    // First lookup, key does not exist, returns false, sets key
    let exists = client.lookup_key();
    assert_eq!(exists, false);

    // Second lookup, key exists, returns true
    let exists2 = client.lookup_key();
    assert_eq!(exists2, true);
}

#![no_std]
use soroban_sdk::{contract, contractimpl, Bytes, BytesN, Env};

#[contract]
pub struct CryptoInLoopFixed;

#[contractimpl]
impl CryptoInLoopFixed {
    pub fn verify_loop(env: Env, pk: BytesN<32>, msg: Bytes, sig: BytesN<64>) {
        for _ in 0..5 {
            env.crypto().ed25519_verify(&pk, &msg, &sig);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{Bytes, BytesN, Env};

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_verify_loop_invalid_sig() {
        let env = Env::default();
        let contract_id = env.register_contract(None, CryptoInLoopFixed);
        let client = CryptoInLoopFixedClient::new(&env, &contract_id);

        let pk = BytesN::from_array(&env, &[0; 32]);
        let msg = Bytes::new(&env);
        let sig = BytesN::from_array(&env, &[0; 64]);

        client.verify_loop(&pk, &msg, &sig);
    }
}

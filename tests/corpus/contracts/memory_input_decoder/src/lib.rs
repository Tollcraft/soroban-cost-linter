#![no_std]
use soroban_sdk::{contract, contractimpl, Symbol, Vec};

#[contract]
pub struct MemoryInputDecoderContract;

#[contractimpl]
impl MemoryInputDecoderContract {
    pub fn verify_checksums(_env: soroban_sdk::Env, payload: Vec<u32>, expected: u32) -> bool {
        let mut sum = 0u32;
        for i in 0..payload.len() {
            if let Some(val) = payload.get(i) {
                sum = sum.wrapping_add(val);
            }
        }
        sum == expected
    }

    pub fn get_action_tag(env: soroban_sdk::Env) -> Symbol {
        Symbol::new(&env, "decode")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{vec, Env, Symbol};

    #[test]
    fn test_verify_checksums_valid() {
        let env = Env::default();
        let payload = vec![&env, 10, 20, 30];
        assert!(MemoryInputDecoderContract::verify_checksums(env.clone(), payload, 60));
    }

    #[test]
    fn test_verify_checksums_invalid() {
        let env = Env::default();
        let payload = vec![&env, 10, 20, 30];
        assert!(!MemoryInputDecoderContract::verify_checksums(env.clone(), payload, 99));
    }

    #[test]
    fn test_verify_checksums_empty() {
        let env = Env::default();
        let payload = vec![&env];
        assert!(MemoryInputDecoderContract::verify_checksums(env.clone(), payload, 0));
    }

    #[test]
    fn test_verify_checksums_wrapping() {
        let env = Env::default();
        // Test wrapping behavior directly. u32::MAX + 1 wraps to 0.
        let payload = vec![&env, u32::MAX, 1];
        assert!(MemoryInputDecoderContract::verify_checksums(env.clone(), payload, 0));
    }

    #[test]
    fn test_get_action_tag() {
        let env = Env::default();
        let tag = MemoryInputDecoderContract::get_action_tag(env.clone());
        assert_eq!(tag, Symbol::new(&env, "decode"));
    }
}

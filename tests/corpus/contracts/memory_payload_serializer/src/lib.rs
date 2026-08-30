#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Bytes, Env, Symbol, Vec};

const PAYLOAD_TOPIC: Symbol = symbol_short!("payload");

#[contract]
pub struct MemoryPayloadSerializerContract;

#[contractimpl]
impl MemoryPayloadSerializerContract {
    pub fn build_payload(env: Env, items: Vec<Bytes>) -> Bytes {
        let mut result = Bytes::new(&env);
        for item in items.iter() {
            result.append(&item);
        }
        env.events().publish((PAYLOAD_TOPIC,), result.clone());
        result
    }

    pub fn format_message(env: Env, header: Bytes, body: Bytes) -> Bytes {
        let mut full_msg = header;
        full_msg.append(&body);
        let _tag = Symbol::new(&env, "header");
        full_msg
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Events as _;
    use soroban_sdk::{vec, Bytes, Env};

    fn client(env: &Env) -> MemoryPayloadSerializerContractClient<'_> {
        let id = env.register(MemoryPayloadSerializerContract, ());
        MemoryPayloadSerializerContractClient::new(env, &id)
    }

    fn bytes(env: &Env, data: &[u8]) -> Bytes {
        Bytes::from_slice(env, data)
    }

    /// Empty input path: no items yields an empty payload.
    #[test]
    fn build_payload_of_no_items_is_empty() {
        let env = Env::default();
        let out = client(&env).build_payload(&vec![&env]);
        assert_eq!(out.len(), 0);
    }

    /// A single item is returned unchanged.
    #[test]
    fn build_payload_single_item_round_trips() {
        let env = Env::default();
        let out = client(&env).build_payload(&vec![&env, bytes(&env, b"abc")]);
        assert_eq!(out, bytes(&env, b"abc"));
    }

    /// Items are concatenated in order, including empty ones.
    #[test]
    fn build_payload_concatenates_in_order() {
        let env = Env::default();
        let items = vec![
            &env,
            bytes(&env, b"ab"),
            bytes(&env, b""),
            bytes(&env, b"cd"),
            bytes(&env, &[0u8, 255u8]),
        ];
        let out = client(&env).build_payload(&items);
        assert_eq!(out, bytes(&env, &[b'a', b'b', b'c', b'd', 0, 255]));
        assert_eq!(out.len(), 6);
    }

    /// The payload is published as exactly one event.
    #[test]
    fn build_payload_publishes_one_event() {
        let env = Env::default();
        client(&env).build_payload(&vec![&env, bytes(&env, b"x")]);
        assert_eq!(env.events().all().events().len(), 1);
    }

    /// The header is followed by the body.
    #[test]
    fn format_message_appends_body_to_header() {
        let env = Env::default();
        let out = client(&env).format_message(&bytes(&env, b"H:"), &bytes(&env, b"body"));
        assert_eq!(out, bytes(&env, b"H:body"));
    }

    /// Empty header and/or body edge cases.
    #[test]
    fn format_message_handles_empty_parts() {
        let env = Env::default();
        let c = client(&env);
        let empty = bytes(&env, b"");
        assert_eq!(c.format_message(&empty, &bytes(&env, b"b")), bytes(&env, b"b"));
        assert_eq!(c.format_message(&bytes(&env, b"h"), &empty), bytes(&env, b"h"));
        assert_eq!(c.format_message(&empty, &empty).len(), 0);
    }

    /// `format_message` emits no events, unlike `build_payload`.
    #[test]
    fn format_message_publishes_no_events() {
        let env = Env::default();
        client(&env).format_message(&bytes(&env, b"h"), &bytes(&env, b"b"));
        assert_eq!(env.events().all().events().len(), 0);
    }
}

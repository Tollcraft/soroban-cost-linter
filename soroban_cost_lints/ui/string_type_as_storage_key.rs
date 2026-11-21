#![warn(string_type_as_storage_key)]
#![allow(storage_write_without_read)]
#![allow(discarded_storage_read)]
#![allow(soroban_storage_in_loop)]
#![allow(blind_storage_write)]
#![allow(storage_read_modify_write)]
#![allow(soroban_redundant_storage_read)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    pub struct Env;

    pub struct String;
    impl String {
        pub fn from_str(_env: &Env, _text: &str) -> String {
            String
        }
    }

    pub struct Symbol;

    pub struct Storage;
    impl Storage {
        pub fn instance(&self) -> Instance {
            Instance
        }
    }

    pub struct Instance;
    impl Instance {
        pub fn get<K, V>(&self, _key: &K) -> Option<V> {
            None
        }
        pub fn has<K>(&self, _key: &K) -> bool {
            false
        }
        pub fn set<K, V>(&self, _key: &K, _value: &V) {}
    }
}

use soroban_sdk::{Env, Instance, String, Symbol};

fn bad_set(env: Env, storage: Instance) {
    storage.set(&String::from_str(&env, "count"), &1u32); //~ WARNING String storage key built from a short literal where a Symbol would do
}

fn bad_get(env: Env, storage: Instance) {
    let _value: Option<u32> = storage.get(&String::from_str(&env, "counter")); //~ WARNING String storage key built from a short literal where a Symbol would do
}

fn bad_has(env: Env, storage: Instance) {
    let _present = storage.has(&String::from_str(&env, "active")); //~ WARNING String storage key built from a short literal where a Symbol would do
}

fn good_long_literal(env: Env, storage: Instance) {
    let _present = storage.has(&String::from_str(&env, "a_very_long_key"));
}

fn good_symbol_key(storage: Instance, symbol: Symbol) {
    let _present = storage.has(&symbol);
}

fn good_integer_key(storage: Instance) {
    let _present = storage.has(&42u32);
}

fn good_string_value(env: Env, storage: Instance, symbol: Symbol) {
    storage.set(&symbol, &String::from_str(&env, "value"));
}

fn good_runtime_key(env: Env, storage: Instance, name: &str) {
    let _present = storage.has(&String::from_str(&env, name));
}

#[allow(string_type_as_storage_key)]
fn allowed_short_key(env: Env, storage: Instance) {
    let _present = storage.has(&String::from_str(&env, "count"));
}

fn main() {}

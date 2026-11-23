#![allow(unused)]
#![warn(storage_write_without_read)]

mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn storage(&self) -> Storage {
            Storage
        }
    }

    pub struct Storage;
    impl Storage {
        pub fn instance(&self) -> InstanceStorage {
            InstanceStorage
        }
    }

    pub struct InstanceStorage;
    impl InstanceStorage {
        pub fn get(&self, _key: &i32) -> Option<i32> {
            None
        }
        pub fn set(&self, _key: &i32, _val: &i32) {}
    }
}

use soroban_sdk::Env;

pub fn __constructor(env: &Env) {
    env.storage().instance().set(&1, &100);
}

pub fn initialize(env: &Env) {
    env.storage().instance().set(&1, &100);
}

pub fn init_owner(env: &Env) {
    env.storage().instance().set(&1, &100);
}

pub fn set_admin(env: &Env) {
    env.storage().instance().set(&1, &100);
}

pub fn initiate_auction(env: &Env) {
    env.storage().instance().set(&1, &100);
}

pub fn reinitialise(env: &Env) {
    env.storage().instance().set(&1, &100);
}

fn main() {}

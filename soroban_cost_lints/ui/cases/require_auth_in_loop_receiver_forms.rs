pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn storage(&self) -> storage::Storage {
            storage::Storage
        }
    }

    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
    }

    pub struct String;
    impl String {
        pub fn require_auth(&self) {}
    }

    pub mod storage {
        pub struct Storage;
        impl Storage {
            pub fn instance(&self) -> Instance {
                Instance
            }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K>(&self, _key: &K) -> Option<super::Address> {
                None
            }
        }
    }

    pub struct AddressList;
    impl AddressList {
        pub fn get(&self, _index: usize) -> Option<Address> {
            None
        }
    }
}

use soroban_sdk::{Address, AddressList, Env, String};

fn warns_for_indexed_address(addrs: AddressList) {
    for index in 0..3 {
        addrs.get(index).unwrap().require_auth();
    }
}

#[allow(
    loop_invariant_storage_access,
    soroban_storage_in_loop,
    unwrap_on_storage_get
)]
fn warns_for_storage_loaded_address(env: Env) {
    for _ in 0..3 {
        env.storage().instance().get(&1).unwrap().require_auth();
    }
}

fn warns_for_address_reference(address: &Address) {
    for _ in 0..3 {
        address.require_auth();
    }
}

fn string_receiver_stays_silent(text: &String) {
    for _ in 0..3 {
        text.require_auth();
    }
}

fn main() {}

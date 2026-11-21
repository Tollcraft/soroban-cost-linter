#![warn(collection_len_from_storage)]
#![allow(unwrap_on_storage_get)]
#![allow(storage_write_without_read)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn storage(&self) -> storage::Storage {
            storage::Storage
        }
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
            pub fn get<K, V>(&self, _key: &K) -> Option<V> {
                None
            }
            pub fn has<K>(&self, _key: &K) -> bool {
                false
            }
        }
    }

    pub mod vec {
        pub struct Vec<T>(std::marker::PhantomData<T>);
        impl<T> Vec<T> {
            pub fn new() -> Self {
                Self(std::marker::PhantomData)
            }
            pub fn len(&self) -> u32 {
                0
            }
            pub fn is_empty(&self) -> bool {
                true
            }
            pub fn get(&self, _index: u32) -> Option<&T> {
                None
            }
        }
    }
    pub use self::vec::Vec;

    pub mod map {
        pub struct Map<K, V>(std::marker::PhantomData<(K, V)>);
        impl<K, V> Map<K, V> {
            pub fn len(&self) -> u32 {
                0
            }
            pub fn is_empty(&self) -> bool {
                true
            }
        }
    }
    pub use self::map::Map;
}

use soroban_sdk::{Env, Map, Vec};

fn bad_len(env: Env) -> u32 {
    let key = 1u32;
    env.storage()
        .instance()
        .get::<_, Vec<u32>>(&key) //~ WARNING stored collection loaded from storage only to call len()
        .unwrap()
        .len()
}

fn bad_is_empty(env: Env) -> bool {
    let key = 2u32;
    env.storage()
        .instance()
        .get::<_, Map<u32, u32>>(&key) //~ WARNING stored collection loaded from storage only to call len()
        .expect("present")
        .is_empty()
}

fn good_other_use_of_collection(env: Env) -> u32 {
    let key = 3u32;
    let stored: Vec<u32> = env.storage().instance().get::<_, Vec<u32>>(&key).unwrap();
    stored.len()
}

fn good_scalar_get(env: Env) -> u32 {
    let key = 4u32;
    env.storage().instance().get::<_, u32>(&key).unwrap()
}

fn good_non_storage_option() -> u32 {
    Some(Vec::<u32>::new()).unwrap().len()
}

fn good_plain_len() -> u32 {
    let local = Vec::<u32>::new();
    local.len()
}

#[allow(collection_len_from_storage)]
fn allowed_len(env: Env) -> u32 {
    let key = 5u32;
    env.storage()
        .instance()
        .get::<_, Vec<u32>>(&key)
        .unwrap()
        .len()
}

fn main() {}

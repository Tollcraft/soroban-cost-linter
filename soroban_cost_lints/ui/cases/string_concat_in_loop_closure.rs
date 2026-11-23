#![allow(bytes_append_in_loop, soroban_inefficient_bytes_concat, dead_code, unused_variables)]

pub mod soroban_sdk {
    pub struct Env;

    pub mod string {
        use super::Env;

        pub struct String;
        impl String {
            pub fn from_bytes(_env: &Env, _bytes: &[u8]) -> String {
                String
            }
        }
    }

    pub mod bytes {
        pub struct Bytes(Vec<u8>);

        impl Bytes {
            pub fn new() -> Bytes {
                Bytes(Vec::new())
            }
            pub fn from_slice(value: &[u8]) -> Bytes {
                Bytes(value.to_vec())
            }
            pub fn append(&mut self, other: &mut Bytes) -> &mut Bytes {
                self.0.extend_from_slice(&other.0);
                self
            }
            pub fn as_slice(&self) -> &[u8] {
                &self.0
            }
        }
    }

    pub use self::bytes::Bytes;
    pub use self::string::String;
}

use soroban_sdk::{Bytes, Env, String};

struct SimpleIter {
    items: std::vec::IntoIter<Bytes>,
}

impl SimpleIter {
    fn new(items: Vec<Bytes>) -> Self {
        Self {
            items: items.into_iter(),
        }
    }

    fn next(&mut self) -> Option<Bytes> {
        self.items.next()
    }
}

fn while_let_append_warns(env: Env) {
    let mut iter = SimpleIter::new(vec![Bytes::from_slice(&[1]), Bytes::from_slice(&[2])]);
    let mut bytes = Bytes::new();
    while let Some(mut piece) = iter.next() {
        bytes.append(&mut piece);
        let _ = String::from_bytes(&env, bytes.as_slice()); // Should Warn
    }
}

fn for_each_append_silent(env: Env) {
    let mut bytes = Bytes::new();
    let pieces = vec![Bytes::from_slice(&[1]), Bytes::from_slice(&[2])];
    // Closures are out of scope by design; enclosing_loop matches only syntactic loops.
    pieces.into_iter().for_each(|mut piece| {
        bytes.append(&mut piece);
        let _ = String::from_bytes(&env, bytes.as_slice());
    });
}

fn main() {}

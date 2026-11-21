// UI test fixture for `map_iter_then_lookup`.

pub mod soroban_sdk {
    pub struct Map;
    impl Map {
        pub fn new() -> Map {
            Map
        }
        pub fn keys(&self) -> Vec<i32> {
            Vec::new()
        }
        pub fn get(&self, _k: &i32) -> Option<i32> {
            None
        }
        pub fn get_unchecked(&self, _k: &i32) -> i32 {
            0
        }
        pub fn iter(&self) -> Vec<(i32, i32)> {
            Vec::new()
        }
    }
}

use soroban_sdk::Map;

// Should Warn — iterating keys and calling get() on same map
fn bad_keys_then_get(map: &Map) {
    for k in map.keys() {
        let _ = map.get(&k);
    }
}

// Should Warn — iterating keys and calling get_unchecked() on same map
fn bad_keys_then_get_unchecked(map: &Map) {
    for k in map.keys() {
        let _ = map.get_unchecked(&k);
    }
}

// Good — direct iteration via map.iter()
fn good_map_iter(map: &Map) {
    for (_k, _v) in map.iter() {
        // no warning
    }
}

// Good — iterating keys without indexing the map
fn good_keys_only(map: &Map) {
    for k in map.keys() {
        let _ = k + 1;
    }
}

// Good — looking up in a different map
fn good_different_map(map_a: &Map, map_b: &Map) {
    for k in map_a.keys() {
        let _ = map_b.get(&k);
    }
}

// Good — suppression via #[allow(map_iter_then_lookup)]
#[allow(map_iter_then_lookup)]
fn good_allowed(map: &Map) {
    for k in map.keys() {
        let _ = map.get(&k);
    }
}

fn main() {}

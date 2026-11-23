pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self {
            Env
        }
    }
    impl Env {
        pub fn invoke_contract<T>(&self, _contract: &Address, _func: &Symbol, _args: ()) -> T
        where
            T: Default,
        {
            T::default()
        }
        pub fn try_invoke_contract<T>(
            &self,
            _contract: &Address,
            _func: &Symbol,
            _args: (),
        ) -> Result<T, ()>
        where
            T: Default,
        {
            Ok(T::default())
        }
    }

    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
        pub fn require_auth_for_args(&self, _args: &[i32]) {}
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol {
            Symbol
        }
    }
}

use soroban_sdk::{Address, Env, Symbol};

pub struct TokenClient;

impl TokenClient {
    pub fn transfer(&self, _to: &Address, _amount: i128) {}
}

fn client_call_does_not_reset_auth(addr: Address, client: TokenClient) {
    addr.require_auth(); // first — no warning
    client.transfer(&addr, 100);
    addr.require_auth(); //~ WARNING require_auth already called on this address
}

// The reset is Env-only: env.invoke_contract clears the tracking map, so the second require_auth stays silent.
fn env_invoke_contract_resets_auth(env: Env, target: Address, func: Symbol, addr: Address) {
    addr.require_auth(); // first — no warning
    let _: () = env.invoke_contract(&target, &func, ());
    addr.require_auth(); // stays silent because invoke_contract resets tracking
}

fn main() {}

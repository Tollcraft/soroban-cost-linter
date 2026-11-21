pub mod soroban_sdk {
    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
        pub fn require_auth_for_args(&self, _args: &[i32]) {}
    }
}

use soroban_sdk::Address;

fn warns_in_for_each(addrs: [Address; 3]) {
    addrs.iter().for_each(|address| address.require_auth());
}

fn warns_for_args_in_for_each(addrs: [Address; 3]) {
    addrs
        .iter()
        .for_each(|address| address.require_auth_for_args(&[]));
}

fn call_once(authorize: impl FnOnce()) {
    authorize();
}

fn closure_called_once_stays_silent() {
    let address = Address;
    call_once(|| address.require_auth());
}

fn main() {}

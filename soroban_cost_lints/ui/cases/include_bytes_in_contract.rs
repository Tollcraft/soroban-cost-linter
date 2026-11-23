#![warn(include_bytes_in_contract)]

fn bad_include_bytes() {
    let _ = include_bytes!("include_bytes_in_contract.rs"); //~ WARNING include_bytes! embeds the file in the Wasm data section
}

fn bad_include_str() {
    let _ = include_str!("include_bytes_in_contract.rs"); //~ WARNING include_str! embeds the file in the Wasm data section
}

fn good_concat() {
    let _ = concat!("foo", "bar");
}

macro_rules! my_macro {
    ($e:expr) => {
        $e
    };
}

fn good_custom_macro() {
    let _ = my_macro!("hello");
}

fn good_small_array() {
    let _small: [u8; 4] = [1, 2, 3, 4];
}

#[allow(include_bytes_in_contract)]
fn allowed_include() {
    let _ = include_bytes!("include_bytes_in_contract.rs");
    let _ = include_str!("include_bytes_in_contract.rs");
}

fn main() {}

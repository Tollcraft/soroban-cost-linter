#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

/// Maximum length for a short symbol (9 characters).
/// Symbols longer than this cannot use the `symbol_short!` macro.
const MAX_SHORT_SYMBOL_LEN: u32 = 9;

/// Contract demonstrating symbol key boundary conditions in Soroban.
///
/// Soroban symbols have two representations:
/// - Short symbols (≤9 ASCII chars): stored inline, created with `symbol_short!`
/// - Long symbols (>9 chars or non-ASCII): stored in host, created with `Symbol::new`
///
/// This contract provides test fixtures for the `symbol_key_boundary` lint,
/// which detects when symbol keys cross the 9-character boundary.
#[contract]
pub struct SymbolKeyBoundaryContract;

#[contractimpl]
impl SymbolKeyBoundaryContract {
    /// Tests various symbol creation patterns at the boundary.
    ///
    /// This function exercises both short and long symbol creation to
    /// verify lint behavior around the 9-character boundary.
    pub fn test_symbols(env: Env) {
        // Short symbols (≤9 chars) - can use symbol_short! macro
        let _s1 = Self::create_short_symbol(&env, "short_9");
        let _s2 = Self::create_short_symbol(&env, "a");

        // Long symbols (>9 chars) - must use Symbol::new
        let _l1 = Self::create_long_symbol(&env, "longer_than_nine");

        // Macro-created short symbol
        let _m1 = symbol_short!("short_9");
    }

    /// Creates a short symbol using the efficient `symbol_short!` macro.
    ///
    /// Use this for compile-time known strings ≤9 ASCII characters.
    /// The macro creates the symbol at compile time with zero runtime cost.
    ///
    /// # Parameters
    /// - `_env`: The Soroban environment (unused for short symbols).
    /// - `value`: A string literal ≤9 ASCII characters.
    ///
    /// # Returns
    /// A `Symbol` suitable for use as a storage key.
    fn create_short_symbol(_env: &Env, value: &'static str) -> Symbol {
        // In practice, use the macro directly: symbol_short!("value")
        // This helper demonstrates the concept for testing.
        Symbol::new(_env, value)
    }

    /// Creates a long symbol using `Symbol::new`.
    ///
    /// Required for strings >9 characters or containing non-ASCII.
    /// This incurs a host function call at runtime.
    ///
    /// # Parameters
    /// - `env`: The Soroban environment.
    /// - `value`: A string >9 characters or with non-ASCII.
    ///
    /// # Returns
    /// A `Symbol` suitable for use as a storage key.
    fn create_long_symbol(env: &Env, value: &str) -> Symbol {
        Symbol::new(env, value)
    }
}
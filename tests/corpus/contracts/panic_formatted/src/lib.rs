//! Test fixture for the `formatted_panic_payload` lint.
//!
//! This contract intentionally demonstrates code that triggers the
//! `formatted_panic_payload` lint by using `panic!` with formatted arguments.
//!
//! ## Purpose
//!
//! The `formatted_panic_payload` lint detects uses of Rust's `core::fmt`
//! formatting machinery (via `format!`, formatted `panic!`, or
//! `.expect(&format!(...))`) in `#![no_std]` Soroban contracts. Including
//! this machinery inflates the compiled WASM binary size (a cost paid on
//! every deploy) and consumes CPU instructions at runtime when the formatting
//! code executes.
//!
//! This test fixture provides a minimal contract that triggers the lint,
//! allowing the linter's test suite to verify that it correctly detects
//! formatted panic usage.
//!
//! ## What This Contract Does
//!
//! The `trigger` function accepts a `u32` parameter and immediately panics
//! with a formatted message that includes that parameter. This is exactly
//! the pattern the lint is designed to catch in production contract code.
//!
//! ## Expected Lint Behavior
//!
//! When `cargo-cost-lint` analyzes this contract, it should flag the
//! `panic!("formatted panic: {}", x)` call with a warning, recommending
//! that the developer use a `#[contracterror]` enum and `panic_with_error!`
//! instead.

#![no_std]
use soroban_sdk::{contract, contractimpl};

/// A minimal Soroban contract used to test lint detection.
///
/// This contract exists solely as a test fixture and is not intended for
/// deployment. It contains deliberately problematic code patterns that the
/// linter should flag.
#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Triggers a formatted panic to test the `formatted_panic_payload` lint.
    ///
    /// This function intentionally uses `panic!` with a format string and
    /// argument, which pulls in Rust's `core::fmt` formatting machinery.
    /// In a real Soroban contract, this would inflate the WASM binary size
    /// and consume unnecessary CPU instructions.
    ///
    /// # Arguments
    ///
    /// * `x` - A value that is interpolated into the panic message via
    ///   format string expansion, demonstrating the anti-pattern.
    ///
    /// # Panics
    ///
    /// Always panics with a formatted message. This is the intended behavior
    /// for testing purposes.
    ///
    /// # Lint Detection
    ///
    /// The linter should flag this function with a `formatted_panic_payload`
    /// warning, suggesting the use of `#[contracterror]` and
    /// `panic_with_error!` instead.
    pub fn trigger(x: u32) {
        panic!("formatted panic: {}", x);
    }
}

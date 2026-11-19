//! Runs the unit tests embedded in `build.rs`.
//!
//! Cargo never compiles a build script with `--test`, so the `#[cfg(test)]`
//! module at the bottom of `build.rs` would otherwise be dead. Pulling the file
//! in as a module lets `cargo test` build and run it; `main` is unused here.
#![allow(dead_code)]

#[path = "../build.rs"]
mod build_script;

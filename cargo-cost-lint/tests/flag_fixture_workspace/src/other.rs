//! Second finding, in its own file, so `.lintignore` and `--diff-only` can
//! remove exactly one of the fixture's two findings.

use crate::soroban_sdk::Env;

/// `blind_storage_write` finding in `src/other.rs`: the second write discards
/// the first store without reading it back.
pub fn other_finding(env: Env) {
    let _ = env.storage().instance().get::<u32, u32>(&2);
    env.storage().instance().set(&2, &4);
    env.storage().instance().set(&2, &5);
}

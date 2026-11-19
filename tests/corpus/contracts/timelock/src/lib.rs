//! Timelock corpus fixture.
//!
//! A small Soroban contract that lets a user lock token amounts until a
//! ledger timestamp and later release them. It exists to give the corpus
//! linter realistic storage, `Map` and loop patterns; several entry points
//! deliberately contain patterns the cost lints flag (redundant `Env`/`Address`
//! clones, host calls inside loops, `Symbol::new` on short literals). The
//! expected findings are recorded in `tests/corpus/baseline.json`, so do not
//! "fix" them here without regenerating that baseline.
//!
//! Storage layout (all in instance storage):
//! - `admin` -> [`Address`]: set once by [`TimelockContract::init`].
//! - `(locked, user)` -> `Map<u64, i128>`: per-user schedule of
//!   `unlock_time -> amount`.
//! - `tot_lock` -> `i128`: running total of every amount ever locked.
//! - `user` -> `i128`: balance credited by [`TimelockContract::release_all`].
#![no_std]
use soroban_sdk::symbol_short;
use soroban_sdk::{contract, contractimpl, vec, Address, Env, Map, Symbol, Vec};

/// Storage key prefix for a user's lock schedule; paired with the user's
/// [`Address`] as `(LOCKED, user)`.
const LOCKED: Symbol = symbol_short!("locked");
/// Storage key for the running total of all locked amounts.
const TOTAL_LOCKED: Symbol = symbol_short!("tot_lock");

/// Timelock contract: holds per-user schedules of amounts that unlock at a
/// given time. See the module docs for the storage layout.
#[contract]
pub struct TimelockContract;

#[contractimpl]
impl TimelockContract {
    /// Records `admin` as the contract administrator.
    ///
    /// This does not check for a previous admin or require auth, so calling it
    /// again overwrites the stored value; the fixture only needs the storage
    /// write, not access control.
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&symbol_short!("admin"), &admin);
    }

    /// Locks `amount` for `user` until `unlock_time`.
    ///
    /// Loads the user's schedule (an empty one if none exists), sets the entry
    /// for `unlock_time` and writes it back. Locking twice for the same
    /// `unlock_time` replaces the earlier amount rather than adding to it.
    /// `amount` is also added to the global total, which is read and rewritten
    /// separately because instance storage has no atomic increment.
    pub fn lock(env: Env, user: Address, amount: i128, unlock_time: u64) {
        let key = (LOCKED, user.clone());
        let existing: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let mut entries = existing;
        entries.set(unlock_time, amount);
        env.storage().instance().set(&key, &entries);

        let total: i128 = env.storage().instance().get(&TOTAL_LOCKED).unwrap_or(0);
        env.storage().instance().set(&TOTAL_LOCKED, &(total + amount));
    }

    /// Releases the amount locked for `user` at exactly `at_time`.
    ///
    /// Removes the entry only when it holds a positive amount; a missing or
    /// zero entry is a silent no-op. The amount is dropped from the schedule
    /// but is neither paid out nor subtracted from the global total.
    /// (Reading the ledger sequence is a stand-in for a real time check and
    /// exercises the host-call lint.)
    pub fn release(env: Env, user: Address, at_time: u64) {
        let key = (LOCKED, user.clone());
        let mut entries: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let amount = entries.get(at_time).unwrap_or(0);
        if amount > 0 {
            entries.remove(at_time);
            env.storage().instance().set(&key, &entries);
            let _seq = env.ledger().sequence();
        }
    }

    /// Credits `user`'s balance for every locked entry whose unlock time is
    /// before `before_time`, and keeps the rest scheduled.
    ///
    /// Entries at or after `before_time` are copied into a fresh map, which is
    /// written back once after the loop. Each released entry reads and writes
    /// the balance inside the loop, an intentionally expensive pattern for the
    /// storage-in-loop lint to find.
    pub fn release_all(env: Env, user: Address, before_time: u64) {
        let key = (LOCKED, user.clone());
        let entries: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let mut remaining = Map::new(&env);
        for (t, amt) in entries.iter() {
            if t >= before_time {
                remaining.set(t, amt);
            } else {
                let balance: i128 = env.storage().instance().get(&user).unwrap_or(0);
                env.storage().instance().set(&user, &(balance + amt));
            }
        }
        env.storage().instance().set(&key, &remaining);
    }

    /// Returns one `"entry"` symbol per lock in `user`'s schedule.
    ///
    /// The result carries no amounts or times; it is a cheap way to see how
    /// many locks exist. The per-iteration ledger read and `Symbol::new` call
    /// are deliberate lint triggers, not needed for the result.
    pub fn summary(env: Env, user: Address) -> Vec<Symbol> {
        let key = (LOCKED, user.clone());
        let entries: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let mut result = vec![&env];
        for (t, amt) in entries.iter() {
            let _seq = env.ledger().sequence();
            let _sym = Symbol::new(&env, "entry");
            result.push_back(_sym);
        }
        result
    }
}

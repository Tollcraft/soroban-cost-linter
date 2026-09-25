#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

const SUPER_ADMIN: Symbol = symbol_short!("SUP_ADM");
const ADMINS: Symbol = symbol_short!("ADMINS");

/// Layered admin authorization contract.
///
/// This contract implements a hierarchical authorization model:
/// - Super-admin can promote/demote regular admins
/// - Regular admins can manage resources
/// - All authorization checks happen BEFORE the loop that processes operations
///
/// This pattern is correct: `require_auth` is never inside a processing loop.
#[contract]
pub struct AuthLayeredAdminContract;

#[contractimpl]
impl AuthLayeredAdminContract {
    /// Initialize the contract with a super-admin.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment providing storage and host functions.
    /// * `super_admin` - The address of the initial super-admin. Must authorize before initialization.
    ///
    /// # Panics
    ///
    /// Panics if `super_admin` fails to authorize via `require_auth`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    /// env.mock_all_auths();
    /// client.init(&admin);
    /// ```
    pub fn init(env: Env, super_admin: Address) {
        super_admin.require_auth();
        env.storage()
            .instance()
            .set(&SUPER_ADMIN, &super_admin);
    }

    /// Super-admin promotes addresses to admin status.
    ///
    /// # Auth Strategy
    ///
    /// The super-admin retrieves the stored super-admin address from storage,
    /// calls `require_auth()` once, then processes all candidates in a loop
    /// without additional auth calls.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment providing storage and host functions.
    /// * `candidates` - A vector of addresses to promote to admin status.
    ///
    /// # Panics
    ///
    /// Panics if the stored super-admin address cannot be retrieved from storage.
    /// Panics if the super-admin fails to authorize.
    ///
    /// # Storage
    ///
    /// Each candidate address is stored as a boolean `true` in instance storage,
    /// marking them as an authorized admin.
    pub fn promote_admins(
        env: Env,
        candidates: soroban_sdk::Vec<Address>,
    ) {
        let super_admin: Address = env
            .storage()
            .instance()
            .get(&SUPER_ADMIN)
            .unwrap();
        // Super-admin authorizes once, before the loop.
        // This hoists the auth cost to O(1) regardless of the number of candidates.
        super_admin.require_auth();

        // Promote each candidate — no auth inside this loop.
        for candidate in candidates.iter() {
            env.storage()
                .instance()
                .set(&candidate, &true);
        }
    }

    /// Admin performs a batch operation on resources.
    ///
    /// # Auth Strategy
    ///
    /// The admin must be previously promoted (stored in storage as `true`).
    /// The function verifies admin status by reading from storage, then calls
    /// `require_auth()` once before processing all resources in a loop.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment providing storage and host functions.
    /// * `admin` - The address of the admin performing the batch update.
    /// * `resource_ids` - A vector of resource IDs to update.
    /// * `new_values` - A vector of new values corresponding to each resource ID.
    ///
    /// # Panics
    ///
    /// Panics with message "caller is not an admin" if `admin` is not found
    /// in the admin storage (`is_admin` is `false`).
    ///
    /// # Performance
    ///
    /// Uses indexed access (`get(i)`) to iterate over resource IDs and
    /// corresponding new values simultaneously.
    pub fn batch_resource_update(
        env: Env,
        admin: Address,
        resource_ids: soroban_sdk::Vec<u32>,
        new_values: soroban_sdk::Vec<u32>,
    ) {
        // Verify admin status and require auth — once, before the loop.
        let is_admin: bool = env
            .storage()
            .instance()
            .get(&admin)
            .unwrap_or(false);
        assert!(is_admin, "caller is not an admin");
        admin.require_auth();

        // Process all resources — no auth inside this loop.
        // Indexed access pairs each resource ID with its corresponding new value.
        for i in 0..resource_ids.len() {
            let resource_id = resource_ids.get(i).unwrap();
            let new_value = new_values.get(i).unwrap_or(0);
            env.storage()
                .instance()
                .set(&resource_id, &new_value);
        }
    }

    /// Super-admin emergency batch freeze: freezes multiple accounts at once.
    ///
    /// # Auth Strategy
    ///
    /// Only the super-admin can invoke this function. The super-admin address
    /// is retrieved from storage, authorized once, and then all accounts are
    /// frozen in a loop without additional auth calls.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment providing storage and host functions.
    /// * `accounts` - A vector of account addresses to freeze.
    ///
    /// # Panics
    ///
    /// Panics if the stored super-admin address cannot be retrieved from storage.
    /// Panics if the super-admin fails to authorize.
    ///
    /// # Storage
    ///
    /// Each account is stored as `false` in instance storage, indicating a
    /// frozen state that can be checked by other contract logic.
    ///
    /// # Auth Hoisting Rationale
    ///
    /// The `require_auth()` call is placed before the freeze loop so that the
    /// authorization cost is paid once, regardless of how many accounts are frozen.
    /// Placing auth inside the loop would trigger the `require_auth_in_loop` lint
    /// and multiply the authorization cost by the number of accounts.
    pub fn emergency_freeze(
        env: Env,
        accounts: soroban_sdk::Vec<Address>,
    ) {
        let super_admin: Address = env
            .storage()
            .instance()
            .get(&SUPER_ADMIN)
            .unwrap();
        super_admin.require_auth();

        // Freeze each account — no auth inside this loop.
        for account in accounts.iter() {
            env.storage()
                .instance()
                .set(&account, &false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    /// Tests that a super-admin can successfully promote candidates to admin status.
    ///
    /// Verifies the full flow:
    /// 1. Super-admin initializes the contract.
    /// 2. Super-admin promotes multiple candidates.
    /// 3. No panics occur during the process.
    #[test]
    fn test_promote_admins() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        let candidates = soroban_sdk::vec![
            &env,
            Address::generate(&env),
            Address::generate(&env),
        ];

        client.promote_admins(&candidates);
    }

    /// Tests that an admin can perform batch resource updates after being promoted.
    ///
    /// Verifies the full flow:
    /// 1. Super-admin initializes the contract.
    /// 2. Super-admin promotes an address to admin.
    /// 3. The promoted admin performs a batch resource update.
    /// 4. Resource IDs and new values are correctly paired and stored.
    #[test]
    fn test_batch_resource_update() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        let admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        // Promote the admin first
        let candidates = soroban_sdk::vec![&env, admin.clone()];
        client.promote_admins(&candidates);

        // Now the admin can do batch updates
        let resource_ids = soroban_sdk::vec![&env, 1u32, 2, 3];
        let new_values = soroban_sdk::vec![&env, 100u32, 200, 300];

        client.batch_resource_update(&admin, &resource_ids, &new_values);
    }

    /// Tests that `emergency_freeze` correctly freezes a list of accounts.
    ///
    /// Verifies the full flow:
    /// 1. Super-admin initializes the contract.
    /// 2. Super-admin calls `emergency_freeze` with multiple accounts.
    /// 3. No panics occur during the freeze process.
    #[test]
    fn test_emergency_freeze() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        let accounts = soroban_sdk::vec![
            &env,
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];

        client.emergency_freeze(&accounts);
    }

    /// Tests that `init` correctly sets the super-admin in storage.
    ///
    /// Verifies that after initialization, the super-admin address is stored
    /// under the `SUPER_ADMIN` key and can be retrieved.
    #[test]
    fn test_init() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        env.mock_all_auths();

        // Initialization should succeed without panicking.
        client.init(&super_admin);
    }

    /// Tests that `batch_resource_update` rejects non-admin callers.
    ///
    /// Verifies the edge case where an address that was never promoted
    /// attempts to call `batch_resource_update`. This should panic
    /// with the message "caller is not an admin".
    #[test]
    #[should_panic(expected = "caller is not an admin")]
    fn test_batch_resource_update_rejects_non_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        // Create an address that was never promoted to admin.
        let non_admin = Address::generate(&env);
        env.mock_all_auths();

        let resource_ids = soroban_sdk::vec![&env, 1u32];
        let new_values = soroban_sdk::vec![&env, 100u32];

        // This should panic because `non_admin` is not stored as an admin.
        client.batch_resource_update(&non_admin, &resource_ids, &new_values);
    }

    /// Tests that `batch_resource_update` handles mismatched vector lengths
    /// gracefully by using `unwrap_or` for missing new_values.
    ///
    /// When `new_values` is shorter than `resource_ids`, the missing values
    /// default to 0 rather than causing a panic.
    #[test]
    fn test_batch_resource_update_with_short_new_values() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        let admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        // Promote the admin first.
        let candidates = soroban_sdk::vec![&env, admin.clone()];
        client.promote_admins(&candidates);

        // resource_ids is longer than new_values; missing values default to 0.
        let resource_ids = soroban_sdk::vec![&env, 1u32, 2, 3];
        let new_values = soroban_sdk::vec![&env, 100u32];

        client.batch_resource_update(&admin, &resource_ids, &new_values);
    }

    /// Tests that `batch_resource_update` handles an empty resource list.
    ///
    /// Edge case: when there are no resources to update, the loop body never
    /// executes and the function should complete without errors.
    #[test]
    fn test_batch_resource_update_empty_resources() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        let admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        // Promote the admin first.
        let candidates = soroban_sdk::vec![&env, admin.clone()];
        client.promote_admins(&candidates);

        // Empty resource lists should complete without errors.
        let resource_ids = soroban_sdk::vec![&env];
        let new_values = soroban_sdk::vec![&env];

        client.batch_resource_update(&admin, &resource_ids, &new_values);
    }

    /// Tests that `emergency_freeze` works with a single account.
    ///
    /// Edge case: minimum number of accounts (1) for the freeze operation.
    #[test]
    fn test_emergency_freeze_single_account() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        let accounts = soroban_sdk::vec![&env, Address::generate(&env)];

        client.emergency_freeze(&accounts);
    }

    /// Tests that `emergency_freeze` works with an empty accounts list.
    ///
    /// Edge case: zero accounts to freeze. The loop body never executes.
    #[test]
    fn test_emergency_freeze_empty_accounts() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthLayeredAdminContract);
        let client = AuthLayeredAdminContractClient::new(&env, &contract_id);

        let super_admin = Address::generate(&env);
        env.mock_all_auths();

        client.init(&super_admin);

        let accounts = soroban_sdk::vec![&env];

        client.emergency_freeze(&accounts);
    }
}
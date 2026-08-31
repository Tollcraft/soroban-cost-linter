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
    pub fn init(env: Env, super_admin: Address) {
        super_admin.require_auth();
        env.storage()
            .instance()
            .set(&SUPER_ADMIN, &super_admin);
    }

    /// Super-admin promotes addresses to admin status.
    /// Auth is checked once, then the loop processes the promotions.
    pub fn promote_admins(
        env: Env,
        candidates: soroban_sdk::Vec<Address>,
    ) {
        let super_admin: Address = env
            .storage()
            .instance()
            .get(&SUPER_ADMIN)
            .unwrap();
        // Super-admin authorizes once, before the loop
        super_admin.require_auth();

        // Promote each candidate — no auth inside this loop
        for candidate in candidates.iter() {
            env.storage()
                .instance()
                .set(&candidate, &true);
        }
    }

    /// Admin performs a batch operation on resources.
    /// The admin authorizes once, then processes all resources in a loop.
    pub fn batch_resource_update(
        env: Env,
        admin: Address,
        resource_ids: soroban_sdk::Vec<u32>,
        new_values: soroban_sdk::Vec<u32>,
    ) {
        // Verify admin status and require auth — once, before the loop
        let is_admin: bool = env
            .storage()
            .instance()
            .get(&admin)
            .unwrap_or(false);
        assert!(is_admin, "caller is not an admin");
        admin.require_auth();

        // Process all resources — no auth inside this loop
        for i in 0..resource_ids.len() {
            let resource_id = resource_ids.get(i).unwrap();
            let new_value = new_values.get(i).unwrap_or(0);
            env.storage()
                .instance()
                .set(&resource_id, &new_value);
        }
    }

    /// Super-admin emergency batch freeze: freezes multiple accounts at once.
    /// Auth is hoisted before the freeze loop.
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

        // Freeze each account — no auth inside this loop
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
}

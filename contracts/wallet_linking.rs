#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, Env, Vec, Symbol
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    UserWallets(Address),
    WalletOwner(Address),
}

#[contract]
pub struct WalletLinkingContract;

#[contractimpl]
impl WalletLinkingContract {

    // 🔗 Link wallet to user identity
    pub fn link_wallet(env: Env, user: Address, wallet: Address) {
        // Require user auth
        user.require_auth();

        // Validate wallet not already linked
        if env.storage().instance().has(&DataKey::WalletOwner(wallet.clone())) {
            panic!("Wallet already linked");
        }

        // Store wallet → user
        env.storage()
            .instance()
            .set(&DataKey::WalletOwner(wallet.clone()), &user);

        // Update user wallet list
        let mut wallets: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::UserWallets(user.clone()))
            .unwrap_or(Vec::new(&env));

        wallets.push_back(wallet.clone());

        env.storage()
            .instance()
            .set(&DataKey::UserWallets(user.clone()), &wallets);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "wallet_linked"), user.clone()),
            wallet
        );
    }

    // ❌ Unlink wallet
    pub fn unlink_wallet(env: Env, user: Address, wallet: Address) {
        user.require_auth();

        let owner: Address = env.storage()
            .instance()
            .get(&DataKey::WalletOwner(wallet.clone()))
            .expect("Wallet not linked");

        if owner != user {
            panic!("Unauthorized unlink attempt");
        }

        // Remove wallet ownership
        env.storage()
            .instance()
            .remove(&DataKey::WalletOwner(wallet.clone()));

        // Remove from user wallet list
        let mut wallets: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::UserWallets(user.clone()))
            .unwrap_or(Vec::new(&env));

        wallets.retain(|w| w != wallet);

        env.storage()
            .instance()
            .set(&DataKey::UserWallets(user.clone()), &wallets);

        env.events().publish(
            (Symbol::new(&env, "wallet_unlinked"), user),
            wallet
        );
    }

    // 📖 Get all wallets for a user
    pub fn get_wallets(env: Env, user: Address) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::UserWallets(user))
            .unwrap_or(Vec::new(&env))
    }

    // 📖 Get owner of wallet
    pub fn get_wallet_owner(env: Env, wallet: Address) -> Option<Address> {
        env.storage()
            .instance()
            .get(&DataKey::WalletOwner(wallet))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_wallet_linking_flow() {
        let env = Env::default();
        let contract_id = env.register_contract(None, WalletLinkingContract);
        let client = WalletLinkingContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let wallet = Address::generate(&env);

        env.mock_all_auths();

        // Link wallet
        client.link_wallet(&user, &wallet);

        // Verify it's linked
        let wallets = client.get_wallets(&user);
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets.get_unchecked(0), wallet.clone());

        let owner = client.get_wallet_owner(&wallet);
        assert_eq!(owner, Some(user.clone()));

        // Unlink wallet
        client.unlink_wallet(&user, &wallet);

        let wallets_after = client.get_wallets(&user);
        assert_eq!(wallets_after.len(), 0);

        let owner_after = client.get_wallet_owner(&wallet);
        assert_eq!(owner_after, None);
    }

    #[test]
    #[should_panic(expected = "Unauthorized unlink attempt")]
    fn test_unauthorized_link_attempt() {
        let env = Env::default();
        let contract_id = env.register_contract(None, WalletLinkingContract);
        let client = WalletLinkingContractClient::new(&env, &contract_id);

        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let wallet = Address::generate(&env);

        env.mock_all_auths();

        client.link_wallet(&user1, &wallet);

        // Unauthorized attempt to unlink
        client.unlink_wallet(&user2, &wallet);
    }
}
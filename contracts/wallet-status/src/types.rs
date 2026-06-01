use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WalletStatus {
    Active,
    Paused,
    Restricted,
}
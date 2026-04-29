use soroban_sdk::{contract, contractimpl, contracttype, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    pub id: u64,
    pub amount: i128,
    pub sender: soroban_sdk::Address,
    pub receiver: soroban_sdk::Address,
    pub timestamp: u64,
    pub export: bool,
}

#[contracttype]
pub enum DataKey {
    Transaction(u64),
}

#[contract]
pub struct TransactionContract;

#[contractimpl]
impl TransactionContract {

    pub fn create_transaction(
        env: Env,
        id: u64,
        amount: i128,
        sender: soroban_sdk::Address,
        receiver: soroban_sdk::Address,
        timestamp: u64,
    ) -> Transaction {
        let tx = Transaction {
            id,
            amount,
            sender,
            receiver,
            timestamp,
            export: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Transaction(id), &tx);

        tx
    }

    pub fn set_export_flag(env: Env, id: u64, export: bool) -> Transaction {
        let mut tx: Transaction = env
            .storage()
            .persistent()
            .get(&DataKey::Transaction(id))
            .expect("Transaction not found");

        tx.export = export;

        env.storage()
            .persistent()
            .set(&DataKey::Transaction(id), &tx);

        tx
    }

    pub fn get_transaction(env: Env, id: u64) -> Transaction {
        env.storage()
            .persistent()
            .get(&DataKey::Transaction(id))
            .expect("Transaction not found")
    }

    pub fn get_export_flag(env: Env, id: u64) -> bool {
        let tx: Transaction = env
            .storage()
            .persistent()
            .get(&DataKey::Transaction(id))
            .expect("Transaction not found");

        tx.export
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_export_flag_defaults_false() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TransactionContract);
        let client = TransactionContractClient::new(&env, &contract_id);

        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);

        let tx = client.create_transaction(&1u64, &1000i128, &sender, &receiver, &1000u64);
        assert_eq!(tx.export, false);
    }

    #[test]
    fn test_set_export_flag_true() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TransactionContract);
        let client = TransactionContractClient::new(&env, &contract_id);

        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);

        client.create_transaction(&2u64, &500i128, &sender, &receiver, &2000u64);
        let updated_tx = client.set_export_flag(&2u64, &true);
        assert_eq!(updated_tx.export, true);
    }

    #[test]
    fn test_set_export_flag_false() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TransactionContract);
        let client = TransactionContractClient::new(&env, &contract_id);

        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);

        client.create_transaction(&3u64, &250i128, &sender, &receiver, &3000u64);
        client.set_export_flag(&3u64, &true);
        let updated_tx = client.set_export_flag(&3u64, &false);
        assert_eq!(updated_tx.export, false);
    }

    #[test]
    fn test_get_export_flag() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TransactionContract);
        let client = TransactionContractClient::new(&env, &contract_id);

        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);

        client.create_transaction(&4u64, &750i128, &sender, &receiver, &4000u64);
        client.set_export_flag(&4u64, &true);

        let flag = client.get_export_flag(&4u64);
        assert_eq!(flag, true);
    }
}

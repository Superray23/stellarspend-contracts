use soroban_sdk::{contracttype, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendingRecord {
    pub transaction_id: u64,
    pub amount: i128,
    pub timestamp: u64,
    pub category: String,
    pub merchant: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportPage {
    pub records: Vec<SpendingRecord>,
    pub total_records: u32,
    pub has_more: bool,
}

pub struct BudgetHistoryEngine;

impl BudgetHistoryEngine {
    /// Queries historical records with an explicit pagination filter window.
    /// Returns a structured `ExportPage` containing chronological ordered items.
    pub fn export_history(
        records: Vec<SpendingRecord>,
        offset: u32,
        limit: u32,
    ) -> ExportPage {
        let total_records = records.len();
        let mut paginated_records = Vec::new(records.env());

        if offset >= total_records {
            return ExportPage {
                records: paginated_records,
                total_records,
                has_more: false,
            };
        }

        // Calculate maximum boundary slice index cleanly without risking out-of-bounds panics
        let end = std::cmp::min(offset + limit, total_records);

        for i in offset..end {
            if let Some(record) = records.get(i) {
                paginated_records.push_back(record);
            }
        }

        let has_more = end < total_records;

        ExportPage {
            records: paginated_records,
            total_records,
            has_more,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, Vec, String};

    #[test]
    fn test_chronological_history_export_pagination() {
        let env = Env::default();
        let mut history_ledger = Vec::new(&env);

        // Generate mock transactional history frames
        for i in 0..5 {
            history_ledger.push_back(SpendingRecord {
                transaction_id: 100 + i as u64,
                amount: 50 + (i as i128 * 10),
                timestamp: 1717200000 + (i as u64 * 3600), // Chronological sequential offsets
                category: String::from_str(&env, "Operations"),
                merchant: String::from_str(&env, "SaaS Supplier"),
            });
        }

        // Test Query Parameter Constraints: Page 1 (Offset=0, Limit=3)
        let page_one = BudgetHistoryEngine::export_history(history_ledger.clone(), 0, 3);
        assert_eq!(page_one.records.len(), 3);
        assert_eq!(page_one.total_records, 5);
        assert!(page_one.has_more);
        assert_eq!(page_one.records.get(0).unwrap().transaction_id, 100);

        // Test Query Parameter Constraints: Page 2 (Offset=3, Limit=3)
        let page_two = BudgetHistoryEngine::export_history(history_ledger, 3, 3);
        assert_eq!(page_two.records.len(), 2); // Pulls remaining 2 records
        assert!(!page_two.has_more);
        assert_eq!(page_two.records.get(0).unwrap().transaction_id, 103);
    }
}
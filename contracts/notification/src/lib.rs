#![no_std]

mod types;
mod errors;

use soroban_sdk::{contract, contractimpl, Env, Vec, String, Symbol};
use types::{Notification, NotificationResult};
use errors::NotificationError;

#[contract]
pub struct NotificationContract;

#[contractimpl]
impl NotificationContract {

    pub fn send_batch_notifications(
        env: Env,
        notifications: Vec<Notification>,
    ) -> Vec<NotificationResult> {

        if notifications.is_empty() {
            panic_with_error!(&env, NotificationError::EmptyBatch);
        }

        let mut results: Vec<NotificationResult> = Vec::new(&env);

        for notification in notifications.iter() {

            let mut success = true;

            // Validate message
            if notification.message.len() == 0 {
                success = false;
            }

            // Validate language (basic example)
            if !Self::is_supported_language(&notification.language) {
                success = false;
            }

            // If valid, emit event
            if success {
                env.events().publish(
                    (Symbol::new(&env, "notification_sent"), notification.recipient.clone()),
                    notification.message.clone()
                );
            }

            results.push_back(NotificationResult {
                recipient: notification.recipient.clone(),
                success,
            });
        }

        results
    }

    fn is_supported_language(lang: &String) -> bool {
        lang == &String::from_str("en")
            || lang == &String::from_str("fr")
            || lang == &String::from_str("es")
            || lang == &String::from_str("de")
    }

    use crate::notifications::budget_notifier::BudgetNotifier;

pub fn update_budget(env: Env, used: i128, limit: i128) {
    BudgetNotifier::check_usage(&env, used, limit);
}

pub fn complete_goal(env: Env) {
    BudgetNotifier::goal_completed(&env);
}

}
use soroban_sdk::{Env, String as SorobanString};

pub fn log_fee_event(env: &Env, label: &str, amount: i128) {
    let _ = env;
    let _ = label;
    let _ = amount;
}

pub fn format_fee_label(prefix: &str, fee_bps: u32) -> std::string::String {
    format!("{}: {} bps", prefix, fee_bps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_label() {
        let label = format_fee_label("transfer_fee", 150);
        assert_eq!(label, "transfer_fee: 150 bps");
    }
}

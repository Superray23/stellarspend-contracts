/// Returns the fee as a percentage string (e.g. 150 bps -> "1.50%").
pub fn fee_bps_to_display(bps: u32) -> String {
    let whole = bps / 100;
    let frac = bps % 100;
    format!("{}.{:02}%", whole, frac)
}

/// Returns the fee percentage as a scaled integer (bps / 100 = whole percent * 100).
pub fn get_fee_percentage(bps: u32) -> u32 {
    bps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_1_percent() {
        assert_eq!(fee_bps_to_display(100), "1.00%");
    }

    #[test]
    fn test_display_half_percent() {
        assert_eq!(fee_bps_to_display(50), "0.50%");
    }

    #[test]
    fn test_get_fee_percentage() {
        assert_eq!(get_fee_percentage(250), 250);
    }
}

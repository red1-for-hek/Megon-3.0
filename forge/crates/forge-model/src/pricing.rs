/// Calculate cost from explicit per-million rates.
/// If rates are None or 0.0, cost is 0.0 (unknown/untracked).
pub fn calculate_cost(
    input_per_million: Option<f64>,
    output_per_million: Option<f64>,
    input_tokens: u32,
    output_tokens: u32,
) -> f64 {
    let inp_rate = input_per_million.unwrap_or(0.0);
    let out_rate = output_per_million.unwrap_or(0.0);
    (input_tokens as f64 / 1_000_000.0) * inp_rate
        + (output_tokens as f64 / 1_000_000.0) * out_rate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_cost_with_rates() {
        let cost = calculate_cost(Some(3.0), Some(15.0), 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.001);
    }

    #[test]
    fn calculate_cost_unknown_rates_is_zero() {
        let cost = calculate_cost(None, None, 999_999, 999_999);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn calculate_cost_partial_rates() {
        // Only input rate known
        let cost = calculate_cost(Some(2.0), None, 1_000_000, 1_000_000);
        assert!((cost - 2.0).abs() < 0.001);
    }

    #[test]
    fn calculate_cost_zero_tokens() {
        let cost = calculate_cost(Some(10.0), Some(30.0), 0, 0);
        assert_eq!(cost, 0.0);
    }
}

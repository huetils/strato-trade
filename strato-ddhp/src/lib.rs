/// Calculates the total delta of the options position.
///
/// # Arguments
///
/// * `delta` - Delta of a single option.
/// * `number_of_contracts` - Number of options contracts.
///
/// # Returns
///
/// The total delta of the options position.
pub fn calculate_total_delta(delta: f64, number_of_contracts: f64) -> f64 {
    delta * number_of_contracts
}

/// Calculates the notional value of the futures contracts needed for hedging.
///
/// # Arguments
///
/// * `total_delta` - Total delta of the options position.
/// * `underlying_price` - Current price of the underlying asset.
///
/// # Returns
///
/// The notional value of the futures contracts.
pub fn calculate_notional_value(total_delta: f64, underlying_price: f64) -> f64 {
    total_delta * underlying_price
}

/// Calculates the required margin for the futures contracts.
///
/// # Arguments
///
/// * `notional_value` - Notional value of the futures contracts.
/// * `leverage` - Leverage ratio (e.g., 10 for 10x leverage).
///
/// # Returns
///
/// The required margin for the futures contracts.
pub fn calculate_required_margin(notional_value: f64, leverage: f64) -> f64 {
    notional_value / leverage
}

/// Calculates the transaction fees for the futures contracts.
///
/// # Arguments
///
/// * `notional_value` - Notional value of the futures contracts.
/// * `transaction_fee_rate` - Transaction fee rate (e.g., 0.001 for 0.1%).
///
/// # Returns
///
/// The transaction fees for the futures contracts.
pub fn calculate_fees(notional_value: f64, transaction_fee_rate: f64) -> f64 {
    notional_value * transaction_fee_rate
}

/// Determines the number of perpetual futures contracts needed to hedge the
/// position.
///
/// # Arguments
///
/// * `current_total_delta` - Current total delta of the options position.
/// * `target_total_delta` - Target total delta (typically zero for
///   delta-neutral).
///
/// # Returns
///
/// The number of perpetual futures contracts to buy or sell to achieve the
/// target delta.
pub fn calculate_perps_needed(current_total_delta: f64, target_total_delta: f64) -> f64 {
    target_total_delta - current_total_delta
}

/// Calculates the number of perpetual futures contracts needed to hedge the
/// options position, along with the required margin and fees.
///
/// # Arguments
///
/// * `current_price` - Current price of the underlying asset.
/// * `current_delta` - Current delta of the options.
/// * `number_of_contracts` - Number of options contracts.
/// * `target_total_delta` - Target total delta (typically zero for
///   delta-neutral).
/// * `leverage` - Leverage ratio (e.g., 10 for 10x leverage).
/// * `transaction_fee_rate` - Transaction fee rate (e.g., 0.001 for 0.1%).
///
/// # Returns
///
/// A tuple containing the number of perpetual futures contracts needed,
/// required margin, and transaction fees.
pub fn get_perps_needed(
    current_price: f64,
    current_delta: f64,
    number_of_contracts: f64,
    target_total_delta: f64,
    leverage: f64,
    transaction_fee_rate: f64,
) -> (f64, f64, f64) {
    let current_total_delta = calculate_total_delta(current_delta, number_of_contracts);
    let perps_needed = calculate_perps_needed(current_total_delta, target_total_delta);
    let notional_value = calculate_notional_value(perps_needed.abs(), current_price);
    let required_margin = calculate_required_margin(notional_value, leverage);
    let fees = calculate_fees(notional_value, transaction_fee_rate);
    (perps_needed, required_margin, fees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total_delta() {
        let delta = 0.25;
        let number_of_contracts = 10.0;
        let expected = 2.5;
        let result = calculate_total_delta(delta, number_of_contracts);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calculate_notional_value() {
        let total_delta = 2.5;
        let underlying_price = 100.0;
        let expected = 250.0;
        let result = calculate_notional_value(total_delta, underlying_price);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calculate_required_margin() {
        let notional_value = 250.0;
        let leverage = 10.0;
        let expected = 25.0;
        let result = calculate_required_margin(notional_value, leverage);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calculate_fees() {
        let notional_value = 250.0;
        let transaction_fee_rate = 0.001;
        let expected = 0.25;
        let result = calculate_fees(notional_value, transaction_fee_rate);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calculate_perps_needed() {
        let current_total_delta = 2.5;
        let target_total_delta = 0.0;
        let expected = -2.5;
        let result = calculate_perps_needed(current_total_delta, target_total_delta);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_perps_needed() {
        let current_price = 100.0;
        let current_delta = 0.25;
        let number_of_contracts = 10.0;
        let target_total_delta = 0.0;
        let leverage = 10.0;
        let transaction_fee_rate = 0.001;

        let expected_perps_needed = -2.5;
        let expected_required_margin = 25.0;
        let expected_fees = 0.25;

        let (perps_needed, required_margin, fees) = get_perps_needed(
            current_price,
            current_delta,
            number_of_contracts,
            target_total_delta,
            leverage,
            transaction_fee_rate,
        );

        assert_eq!(perps_needed, expected_perps_needed);
        assert_eq!(required_margin, expected_required_margin);
        assert_eq!(fees, expected_fees);
    }
}

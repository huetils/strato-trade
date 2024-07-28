/// Calculates the relative bid and ask depths based on the relative half-spread, skew, position,
/// and order quantity.
///
/// This helps set buy and sell prices that adjust to the market and how much you currently own.
/// It can help manage risk and improve profits. For example, in a volatile market, you might set
/// buy prices lower and sell prices higher.
///
/// # Parameters
///
/// - `relative_half_spread`: The base distance from the middle price for buy and sell prices.
/// - `skew`: A factor used to adjust the spread based on the trader's current position. When you
///   have a long position (holding more assets), you might want to buy less and sell more to
///   reduce your position. Skew helps increase the bid depth (lowering the bid price) and decrease
///   the ask depth (raising the ask price) to make it less likely to buy more and more likely to
///   sell. Conversely, when you have a short position (holding fewer assets or negative position),
///   you might want to buy more and sell less to increase your position. Skew helps decrease the
///   bid depth (raising the bid price) and increase the ask depth (lowering the ask price).
///   Use a non-zero skew when you need to manage your position size more carefully, especially in
///   strategies where position size has a significant impact. For high-frequency trading based on
///   order book imbalances, you might set `skew` to `0.0` as positions are held very briefly.
/// - `position`: How much of the asset you currently own. Positive for owning more, negative for
///   owing more.
/// - `order_qty`: The size of each order you want to place.
///
/// # Returns
///
/// A tuple containing the distances from the mid price for the buy and sell prices (relative bid
/// depth and relative ask depth).
///
/// # Example
///
/// ```
/// let relative_half_spread = 0.01; // Base half-spread, set to 1%
/// let skew = 0.01; // Skew factor, set to 1%
/// let position = 10.0; // Current position, set to 10 units
/// let order_qty = 10.0; // Order quantity, set to 10 units
///
/// let (relative_bid_depth, relative_ask_depth) = calculate_relative_depths(
///     relative_half_spread, skew, position, order_qty
/// );
///
/// assert_eq!(relative_bid_depth, 0.02);
/// assert_eq!(relative_ask_depth, 0.0);
/// ```
pub fn calculate_relative_depths(
    relative_half_spread: f64,
    skew: f64,
    position: f64,
    order_qty: f64,
) -> (f64, f64) {
    let normalized_position = position / order_qty;
    let relative_bid_depth = relative_half_spread + skew * normalized_position;
    let relative_ask_depth = relative_half_spread - skew * normalized_position;
    (relative_bid_depth, relative_ask_depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_relative_depths_no_position() {
        let relative_half_spread = 0.01;
        let skew = 0.0;
        let position = 0.0;
        let order_qty = 10.0;

        let (relative_bid_depth, relative_ask_depth) =
            calculate_relative_depths(relative_half_spread, skew, position, order_qty);

        assert_eq!(relative_bid_depth, 0.01);
        assert_eq!(relative_ask_depth, 0.01);
    }

    #[test]
    fn test_calculate_relative_depths_long_position() {
        let relative_half_spread = 0.01;
        let skew = 0.01;
        let position = 10.0;
        let order_qty = 10.0;

        let (relative_bid_depth, relative_ask_depth) =
            calculate_relative_depths(relative_half_spread, skew, position, order_qty);

        assert_eq!(relative_bid_depth, 0.02); // 0.01 + 0.01 * (10 / 10)
        assert_eq!(relative_ask_depth, 0.0); // 0.01 - 0.01 * (10 / 10)
    }

    #[test]
    fn test_calculate_relative_depths_short_position() {
        let relative_half_spread = 0.01;
        let skew = 0.01;
        let position = -10.0;
        let order_qty = 10.0;

        let (relative_bid_depth, relative_ask_depth) =
            calculate_relative_depths(relative_half_spread, skew, position, order_qty);

        assert_eq!(relative_bid_depth, 0.0); // 0.01 + 0.01 * (-10 / 10)
        assert_eq!(relative_ask_depth, 0.02); // 0.01 - 0.01 * (-10 / 10)
    }
}

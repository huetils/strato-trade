/// Enum representing trading signals
#[derive(Debug, PartialEq)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}

/// Trait for trading strategies
pub trait TradingStrategy {
    /// Analyzes the market data and returns a trading signal
    ///
    /// # Arguments
    ///
    /// * `market_data` - A slice of market data values (e.g., prices, volumes)
    ///
    /// # Returns
    ///
    /// A `Signal` indicating whether to buy, sell, or hold
    fn analyze(&self, market_data: &[f64]) -> Signal;
}

/// Example of a simple moving average crossover strategy
pub struct MovingAverageCrossover {
    short_window: usize,
    long_window: usize,
}

impl MovingAverageCrossover {
    /// Creates a new instance of `MovingAverageCrossover`
    ///
    /// # Arguments
    ///
    /// * `short_window` - The window size for the short moving average
    /// * `long_window` - The window size for the long moving average
    ///
    /// # Returns
    ///
    /// A new `MovingAverageCrossover` instance
    pub fn new(short_window: usize, long_window: usize) -> Self {
        MovingAverageCrossover {
            short_window,
            long_window,
        }
    }

    /// Calculates the moving average of the last `window_size` values in `data`
    ///
    /// # Arguments
    ///
    /// * `data` - A slice of data values
    /// * `window_size` - The window size for the moving average
    ///
    /// # Returns
    ///
    /// The moving average
    fn moving_average(data: &[f64], window_size: usize) -> f64 {
        let sum: f64 = data.iter().rev().take(window_size).sum();
        sum / window_size as f64
    }
}

impl TradingStrategy for MovingAverageCrossover {
    fn analyze(&self, market_data: &[f64]) -> Signal {
        if market_data.len() < self.long_window {
            return Signal::Hold;
        }

        let short_ma = Self::moving_average(market_data, self.short_window);
        let long_ma = Self::moving_average(market_data, self.long_window);

        if short_ma > long_ma {
            Signal::Buy
        } else if short_ma < long_ma {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average_crossover() {
        let strategy = MovingAverageCrossover::new(3, 5);

        // Test with insufficient data
        let market_data = vec![1.0, 2.0, 3.0];
        assert_eq!(strategy.analyze(&market_data), Signal::Hold);

        // Test with enough data
        let market_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        assert_eq!(strategy.analyze(&market_data), Signal::Buy);

        let market_data = vec![7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        assert_eq!(strategy.analyze(&market_data), Signal::Sell);

        let market_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert_eq!(strategy.analyze(&market_data), Signal::Hold);
    }

    #[test]
    fn test_moving_average() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = MovingAverageCrossover::moving_average(&data, 3);
        assert_eq!(ma, 4.0);
    }
}

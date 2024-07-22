pub mod ta;
pub mod vars;

#[cfg(test)]
mod tests {
    use crate::ta::atr::atr;
    use crate::ta::rma::rma;
    use crate::ta::sma::sma;
    use crate::vars::ohlc::Ohlc;

    #[test]
    fn test_sma() {
        let src = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let length = 3;
        let expected_sma = vec![0.0, 0.0, 2.0, 3.0, 4.0];
        let sma_values = sma(&src, length);
        assert_eq!(sma_values, expected_sma);
    }

    #[test]
    fn test_rma() {
        let src = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let length = 3;
        let expected_rma = vec![
            2.0,
            2.0,
            2.3333333333333335,
            2.8888888888888893,
            3.592592592592593,
        ];
        let rma_values = rma(&src, length);
        assert_eq!(rma_values.len(), expected_rma.len());
        for (i, &value) in rma_values.iter().enumerate() {
            assert!((value - expected_rma[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn test_atr() {
        let candles = vec![
            Ohlc {
                open: 1.0,
                high: 3.0,
                low: 1.0,
                close: 2.0,
                ..Default::default()
            },
            Ohlc {
                open: 2.0,
                high: 4.0,
                low: 2.0,
                close: 3.0,
                ..Default::default()
            },
            Ohlc {
                open: 3.0,
                high: 5.0,
                low: 3.0,
                close: 4.0,
                ..Default::default()
            },
        ];
        let length = 2;
        let expected_tr = vec![0.0, 2.0, 2.0];
        let expected_atr = rma(&expected_tr.clone(), length);
        let atr_values = atr(&candles, length);
        assert_eq!(atr_values.len(), expected_atr.len());
        for (i, &value) in atr_values.iter().enumerate() {
            assert!((value - expected_atr[i]).abs() < 1e-6);
        }
    }
}

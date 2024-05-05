use std::num::ParseFloatError;

use super::Amount4DecimalBased;

#[derive(Debug)]
struct AmountParseError;
impl Amount4DecimalBased {
    pub fn from_str(s: &str) -> Result<Self, ParseFloatError> {
        let mut v = s.parse::<f64>()?;
        v *= 10_000f64;
        Ok(Self(v as i64))
    }

    fn to_str(&self) -> String {
        let mut f = self.0 as f64;
        f /= 10_000 as f64;
        format!("{:.4}", f)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::model::Amount4DecimalBased;

    #[rstest]
    #[case("0", Amount4DecimalBased(0))]
    #[case("0.0001", Amount4DecimalBased(1))]
    #[case("0.0010", Amount4DecimalBased(10))]
    #[case("0.0100", Amount4DecimalBased(100))]
    #[case("0.1000", Amount4DecimalBased(1_000))]
    #[case("1.0000", Amount4DecimalBased(10_000))]
    #[case("0.001", Amount4DecimalBased(10))]
    #[case("0.0010", Amount4DecimalBased(10))]
    #[case("0.01", Amount4DecimalBased(100))]
    #[case("0.1", Amount4DecimalBased(1_000))]
    #[case("1", Amount4DecimalBased(10_000))]
    #[case("1.01", Amount4DecimalBased(10_100))]
    #[case("10.01", Amount4DecimalBased(100_100))]
    fn deserialsation_works(#[case] input: &str, #[case] expected: Amount4DecimalBased) {
        assert_eq!(Amount4DecimalBased::from_str(input).unwrap(), expected);
    }

    #[rstest]
    #[case(Amount4DecimalBased(0), "0.0000")]
    #[case(Amount4DecimalBased(1), "0.0001")]
    #[case(Amount4DecimalBased(10), "0.0010")]
    #[case(Amount4DecimalBased(100), "0.0100")]
    #[case(Amount4DecimalBased(1_000), "0.1000")]
    #[case(Amount4DecimalBased(10_000), "1.0000")]
    #[case(Amount4DecimalBased(10_100), "1.0100")]
    #[case(Amount4DecimalBased(100_100), "10.0100")]
    fn serialsation_works(#[case] amount: Amount4DecimalBased, #[case] expected: &str) {
        assert_eq!(amount.to_str(), expected);
    }
}

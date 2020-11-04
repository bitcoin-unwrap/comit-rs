use anyhow::Context;
use bitcoin::hashes::core::cmp::Ordering;
use num::BigUint;
use std::str::FromStr;

/// Multiply float by 10e`pow`, Returns as a BigUint. No data loss.
/// Errors if the float is negative.
/// Errors if the result is a fraction.
pub fn multiply_pow_ten(float: &str, pow: u16) -> anyhow::Result<BigUint> {
    {
        // Verify that the input is actually a number
        let str = float.replace('.', &"");
        let _ = BigUint::from_str(&str).context("Expecting a float")?;
    }

    let mut float = float.replace('_', &"");
    let decimal_index = float.find('.');

    match decimal_index {
        None => {
            let zeroes = "0".repeat(pow as usize);
            Ok(BigUint::from_str(&format!("{}{}", float, zeroes)).expect("an integer"))
        }
        Some(decimal_index) => {
            let mantissa = float.split_off(decimal_index + 1);
            // Removes the decimal point
            float.truncate(float.len() - 1);
            let integer = float;

            let pow = pow as usize;
            match mantissa.len().cmp(&pow) {
                Ordering::Less => {
                    let remain = pow as usize - mantissa.len();
                    let zeroes = "0".repeat(remain);
                    Ok(
                        BigUint::from_str(&format!("{}{}{}", integer, mantissa, zeroes))
                            .expect("an integer"),
                    )
                }
                Ordering::Equal => {
                    Ok(BigUint::from_str(&format!("{}{}", integer, mantissa)).expect("an integer"))
                }
                Ordering::Greater => anyhow::bail!("Result is not an integer"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn given_integer_then_it_multiplies() {
        let float = "123_456_789.0";
        let pow = 6;

        assert_eq!(
            multiply_pow_ten(float, pow).unwrap(),
            BigUint::from(123_456_789_000_000u64)
        )
    }

    #[test]
    fn given_mantissa_of_pow_length_then_it_multiplies() {
        let float = "123.123_456_789";
        let pow = 9;

        assert_eq!(
            multiply_pow_ten(float, pow).unwrap(),
            BigUint::from(123_123_456_789u64)
        )
    }

    #[test]
    fn given_mantissa_length_lesser_than_pow_then_it_multiplies() {
        let float = "123.123_456_789";
        let pow = 12;

        assert_eq!(
            multiply_pow_ten(float, pow).unwrap(),
            BigUint::from(123_123_456_789_000u64)
        )
    }

    #[test]
    fn given_mantissa_length_greater_than_pow_then_it_errors() {
        let float = "123.123_456_789";
        let pow = 6;

        assert!(multiply_pow_ten(float, pow).is_err(),)
    }

    #[test]
    fn given_negative_float_then_it_errors() {
        let float = "-123_456_789.0";
        let pow = 6;

        assert!(multiply_pow_ten(float, pow).is_err(),)
    }

    proptest! {
        #[test]
        fn multiple_pow_ten_doesnt_panic(s in any::<String>(), p in any::<u16>()) {
            let _ = multiply_pow_ten(&s, p);
        }
    }

    prop_compose! {
        fn new_biguint()(s in "[0-9]+") -> anyhow::Result<BigUint> {
            Ok(BigUint::from_str(&s)?)
        }
    }
}

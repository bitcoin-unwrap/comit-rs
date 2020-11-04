use crate::{
    bitcoin::{self},
    ethereum, Rate,
};
use ::bitcoin::util::amount::Denomination;
use anyhow::Context;
use comit::{
    asset::{Erc20, Erc20Quantity},
    ethereum::Address,
};
use conquer_once::Lazy;
use num::{BigUint, ToPrimitive, Zero};
use std::convert::{TryFrom, TryInto};

/// As per https://etherscan.io/token/0x2260fac5e5542a773aa44fbcfedf7c193bc2c599
static MAINNET_WBTC_CONTRACT_ADDRESS: Lazy<Address> = Lazy::new(|| {
    "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"
        .parse()
        .expect("Valid hex")
});
/// As per https://kovan.etherscan.io/token/0xa0a5ad2296b38bd3e3eb59aaeaf1589e8d9a29a9
static KOVAN_DAI_CONTRACT_ADDRESS: Lazy<Address> = Lazy::new(|| {
    "0xa0a5ad2296b38bd3e3eb59aaeaf1589e8d9a29a9"
        .parse()
        .expect("Valid hex")
});
/// As per https://rinkeby.etherscan.io/address/0xeba449b9150f34396d529643263a90d495ae563c
static RINKEBY_WBTC_CONTRACT_ADDRESS: Lazy<Address> = Lazy::new(|| {
    "0xEBa449b9150F34396D529643263A90D495Ae563c"
        .parse()
        .expect("Valid hex")
});
/// As per https://ropsten.etherscan.io/address/0x65058d7081fcdc3cd8727dbb7f8f9d52cefdd291
static ROPSTEN_WBTC_CONTRACT_ADDRESS: Lazy<Address> = Lazy::new(|| {
    "0x65058d7081FCdC3cd8727dbb7F8F9D52CefDd291"
        .parse()
        .expect("Valid hex")
});

#[derive(Clone, Copy, Ord, PartialOrd, PartialEq, Eq, Default)]
pub struct Amount(bitcoin::Amount);

impl Amount {
    pub fn zero() -> Self {
        Self(bitcoin::Amount::ZERO)
    }

    /// Rounds the value received to a 9 digits mantissa.
    pub fn from_wbtc(wbtc: f64) -> anyhow::Result<Self> {
        Ok(Self(bitcoin::Amount::from_btc(wbtc)?))
    }

    /// Rounds to 2 digits after decimal point
    pub fn as_wbtc_rounded(&self) -> f64 {
        self.0.as_btc()
    }

    pub fn from_sat(sat: u64) -> Self {
        Self(bitcoin::Amount::from_sat(sat))
    }

    pub fn as_sat(&self) -> u64 {
        self.0.as_sat()
    }

    /// Allow to know the worth of self in bitcoin asset using the given
    /// conversion rate. Truncation may be done during the conversion to
    /// allow a result in satoshi
    pub fn worth_in(&self, btc_to_dai: Rate) -> anyhow::Result<bitcoin::Amount> {
        // TODO: Use Price and NonZeroU64 so it cannot be zero
        if btc_to_dai.significand().is_zero() {
            anyhow::bail!("Cannot use a nil rate.")
        }

        let sats = self.as_sat() as u128;
        let rate = btc_to_dai.significand() as u128;
        let precision_adjustment = 10u128.pow(Rate::PRECISION as u32);

        let sats = (sats * precision_adjustment / rate)
            .to_u64()
            .context("should fit into u64")?;

        Ok(bitcoin::Amount::from_sat(sats))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.as_sat().to_le_bytes().to_vec()
    }
}

pub(super) fn is_mainnet_contract_address(contract_address: Address) -> bool {
    contract_address == *MAINNET_WBTC_CONTRACT_ADDRESS
}

pub(super) fn is_ropsten_contract_address(contract_address: Address) -> bool {
    contract_address == *ROPSTEN_WBTC_CONTRACT_ADDRESS
}

pub(super) fn is_rinkeby_contract_address(contract_address: Address) -> bool {
    contract_address == *RINKEBY_WBTC_CONTRACT_ADDRESS
}

pub(super) fn is_kovan_contract_address(contract_address: Address) -> bool {
    contract_address == *KOVAN_DAI_CONTRACT_ADDRESS
}

pub(super) fn token_contract_address(chain: ethereum::Chain) -> Address {
    use ethereum::Chain::*;
    match chain {
        Mainnet => *MAINNET_WBTC_CONTRACT_ADDRESS,
        Ropsten => *ROPSTEN_WBTC_CONTRACT_ADDRESS,
        Rinkeby => *RINKEBY_WBTC_CONTRACT_ADDRESS,
        Kovan => *KOVAN_DAI_CONTRACT_ADDRESS,
        Local {
            dai_contract_address,
            ..
        } => dai_contract_address,
    }
}

impl std::fmt::Debug for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt_value_in(f, Denomination::Bitcoin)?;
        write!(f, " WBTC")
    }
}

impl std::ops::Add for Amount {
    type Output = Amount;
    fn add(self, rhs: Self) -> Self::Output {
        Amount(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}

impl std::ops::AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0.add_assign(rhs.0)
    }
}

impl std::ops::SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Self) {
        self.0.sub_assign(rhs.0)
    }
}

impl TryFrom<Erc20> for Amount {
    type Error = anyhow::Error;

    fn try_from(value: Erc20) -> Result<Self, Self::Error> {
        value.quantity.try_into()
    }
}

impl TryFrom<Erc20Quantity> for Amount {
    type Error = anyhow::Error;

    fn try_from(value: Erc20Quantity) -> Result<Self, Self::Error> {
        let sats = BigUint::from(value.clone());

        Ok(Amount::from_sat(sats.to_u64().with_context(|| {
            format!("{} does not fit into a u64", value)
        })?))
    }
}

#[cfg(test)]
pub fn dai(dai: f64) -> Amount {
    Amount::from_wbtc(dai).unwrap()
}

#[cfg(test)]
pub fn some_dai(dai: f64) -> Option<Amount> {
    Some(Amount::from_wbtc(dai).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::convert::TryFrom;

    #[test]
    fn given_float_dai_amount_less_precise_than_attodai_then_exact_value_is_stored() {
        let some_dai = Amount::from_wbtc(15.555_555_55).unwrap();
        let same_amount = Amount::from_sat(1_555_555_555u64);

        assert_eq!(some_dai, same_amount);
    }

    #[test]
    fn using_rate_returns_correct_result() {
        let wbtc = Amount::from_wbtc(1.0).unwrap();
        let rate = Rate::try_from(1.0).unwrap();

        let res: bitcoin::Amount = wbtc.worth_in(rate).unwrap();

        let btc = bitcoin::Amount::from_btc(1.0).unwrap();
        assert_eq!(res, btc);
    }

    #[test]
    fn worth_in_result_truncated_1() {
        let dai = Amount::from_wbtc(112.648125).unwrap();
        let rate = Rate::try_from(1.0).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        let btc = bitcoin::Amount::from_btc(112.648125).unwrap();
        assert_eq!(res, btc);
    }

    #[test]
    fn worth_in_result_truncated_2() {
        let dai = Amount::from_wbtc(0.01107).unwrap();
        let rate = Rate::try_from(9000.0).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        let btc = bitcoin::Amount::from_sat(123);
        assert_eq!(res, btc);
    }

    #[test]
    fn given_amount_has_2_digits_after_decimal_return_same_amount() {
        let dai = Amount::from_wbtc(1.23).unwrap();
        let dai = dai.as_wbtc_rounded();

        assert!(dai - 1.23 < 1e-10)
    }

    #[test]
    fn given_amount_has_3_digits_after_decimal_return_rounded_up_amount() {
        let dai = Amount::from_wbtc(1.235).unwrap();
        let dai = dai.as_wbtc_rounded();

        assert!(dai - 1.24 < 1e-10)
    }

    #[test]
    fn given_amount_is_centi_dai_return_centi_dai() {
        let dai = Amount::from_wbtc(0.1).unwrap();
        let dai = dai.as_wbtc_rounded();

        assert!(dai - 0.1 < 1e-10)
    }

    #[test]
    fn given_amount_is_deci_dai_return_deci_dai() {
        let dai = Amount::from_wbtc(0.01).unwrap();
        let dai = dai.as_wbtc_rounded();

        assert!(dai - 0.01 < 1e-10)
    }

    #[test]
    fn given_amount_is_one_atto_dai_as_dai_returns_one_atto_dai() {
        let dai = Amount::from_sat(1);

        assert_eq!(dai.to_string(), "0.00000001 WBTC".to_string())
    }

    #[test]
    fn given_amount_is_one_tenth_of_a_dai_as_dai_returns_one_tenth_of_a_dai() {
        let dai = Amount::from_wbtc(0.1).unwrap();

        assert_eq!(dai.to_string(), "0.10000000 WBTC".to_string())
    }

    #[test]
    fn given_amount_is_one_dai_as_dai_returns_one_dai() {
        let dai = Amount::from_wbtc(1.0).unwrap();

        assert_eq!(dai.to_string(), "1.00000000 WBTC".to_string())
    }

    #[test]
    fn given_amount_is_ten_dai_as_dai_returns_ten_dai() {
        let dai = Amount::from_wbtc(10.0).unwrap();

        assert_eq!(dai.to_string(), "10.00000000 WBTC".to_string())
    }

    proptest! {
        #[test]
        fn worth_in_bitcoin_doesnt_panic(uint in any::<u64>(), r in any::< f64>()) {
            let rate = Rate::try_from(r);
            if let Ok(rate) = rate {
                let amount = Amount::from_sat(uint);
                let _: anyhow::Result<bitcoin::Amount> = amount.worth_in(rate);
            }
        }
    }
}

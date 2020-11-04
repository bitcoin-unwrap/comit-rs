use crate::{
    bitcoin,
    ethereum::{self, wbtc},
    order::Symbol,
    Rate,
};
use comit::{ledger, order::SwapProtocol, BtcDaiOrder, Position, Role};

pub mod strategy;

// Bundles the state of the application
#[derive(Debug)]
pub struct Maker {
    btc_balance: Option<bitcoin::Amount>,
    dai_balance: Option<wbtc::Amount>,
    // TODO: Replace with a Price.
    fixed_rate: Rate,
    pub strategy: strategy::AllIn,
    bitcoin_network: ledger::Bitcoin,
    ethereum_chain: ethereum::Chain,
    role: Role,
    comit_network: comit::Network,
}

impl Maker {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        btc_balance: bitcoin::Amount,
        dai_balance: wbtc::Amount,
        fixed_rate: Rate,
        strategy: strategy::AllIn,
        bitcoin_network: ledger::Bitcoin,
        dai_chain: ethereum::Chain,
        role: Role,
        comit_network: comit::Network,
    ) -> Self {
        Maker {
            btc_balance: Some(btc_balance),
            dai_balance: Some(dai_balance),
            fixed_rate,
            strategy,
            bitcoin_network,
            ethereum_chain: dai_chain,
            role,
            comit_network,
        }
    }

    pub fn update_bitcoin_balance(
        &mut self,
        balance: bitcoin::Amount,
    ) -> anyhow::Result<Option<PublishOrders>> {
        // if we had a balance and the balance did not change => no new orders
        if let Some(previous_balance) = self.btc_balance {
            if previous_balance == balance {
                return Ok(None);
            }
        }

        self.btc_balance = Some(balance);

        Ok(Some(PublishOrders {
            new_sell_order: self.new_sell_order()?,
            new_buy_order: self.new_buy_order()?,
        }))
    }

    pub fn invalidate_bitcoin_balance(&mut self) {
        self.btc_balance = None;
    }

    pub fn update_dai_balance(
        &mut self,
        balance: wbtc::Amount,
    ) -> anyhow::Result<Option<PublishOrders>> {
        // if we had a balance and the balance did not change => no new orders
        if let Some(previous_balance) = self.dai_balance {
            if previous_balance == balance {
                return Ok(None);
            }
        }

        self.dai_balance = Some(balance);

        Ok(Some(PublishOrders {
            new_sell_order: self.new_sell_order()?,
            new_buy_order: self.new_buy_order()?,
        }))
    }

    pub fn invalidate_dai_balance(&mut self) {
        self.dai_balance = None;
    }

    pub fn swap_protocol(&self, position: Position) -> SwapProtocol {
        SwapProtocol::new(self.role, position, self.comit_network)
    }

    pub fn new_sell_order(&self) -> anyhow::Result<BtcDaiOrder> {
        let btc_balance = self
            .btc_balance
            .ok_or_else(|| BalanceNotAvailable(Symbol::Btc))?;

        let form = self.strategy.new_sell(btc_balance, self.fixed_rate)?;
        let order = form.to_comit_order(self.swap_protocol(Position::Sell));

        Ok(order)
    }

    pub fn new_buy_order(&self) -> anyhow::Result<BtcDaiOrder> {
        let dai_balance = self
            .dai_balance
            .clone()
            .ok_or_else(|| BalanceNotAvailable(Symbol::Dai))?;

        let form = self.strategy.new_buy(dai_balance, self.fixed_rate)?;
        let order = form.to_comit_order(self.swap_protocol(Position::Buy));

        Ok(order)
    }

    pub fn process_taken_order(
        &mut self,
        order: BtcDaiOrder,
    ) -> anyhow::Result<TakeRequestDecision> {
        let dai_balance = self
            .dai_balance
            .as_ref()
            .ok_or_else(|| BalanceNotAvailable(Symbol::Dai))?;
        let btc_balance = self
            .btc_balance
            .as_ref()
            .ok_or_else(|| BalanceNotAvailable(Symbol::Btc))?;

        self.strategy
            .process_taken_order(order, self.fixed_rate, dai_balance, btc_balance)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TakeRequestDecision {
    GoForSwap,
    RateNotProfitable,
    InsufficientFunds,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PublishOrders {
    pub new_sell_order: BtcDaiOrder,
    pub new_buy_order: BtcDaiOrder,
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Rate not available when trying to create new {0} order.")]
pub struct RateNotAvailable(Position);

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("{0} balance not available.")]
pub struct BalanceNotAvailable(Symbol);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bitcoin,
        bitcoin::amount::{btc, some_btc},
        ethereum::wbtc::{dai, some_dai},
        order::btc_dai_order,
        rate::rate,
        Rate, Spread, StaticStub,
    };
    use std::convert::{TryFrom, TryInto};

    impl StaticStub for Maker {
        fn static_stub() -> Self {
            Self {
                btc_balance: Some(bitcoin::Amount::default()),
                dai_balance: Some(wbtc::Amount::default()),
                strategy: strategy::AllIn::static_stub(),
                fixed_rate: Rate::static_stub(),
                bitcoin_network: ledger::Bitcoin::Mainnet,
                ethereum_chain: ethereum::Chain::static_stub(),
                role: Role::Bob,
                comit_network: comit::Network::Main,
            }
        }
    }

    #[test]
    fn fail_to_confirm_sell_order_if_sell_rate_is_not_good_enough() {
        let mut maker = Maker {
            fixed_rate: 10000.0.try_into().unwrap(),
            ..StaticStub::static_stub()
        };

        let taken_order = btc_dai_order(Position::Sell, btc(1.0), rate(9000.0));

        let result = maker.process_taken_order(taken_order).unwrap();

        assert_eq!(result, TakeRequestDecision::RateNotProfitable);
    }

    #[test]
    fn fail_to_confirm_buy_order_if_buy_rate_is_not_good_enough() {
        let mut maker = Maker {
            fixed_rate: 10000.0.try_into().unwrap(),
            ..StaticStub::static_stub()
        };

        let taken_order = btc_dai_order(Position::Buy, btc(1.0), rate(11000.0));

        let result = maker.process_taken_order(taken_order).unwrap();

        assert_eq!(result, TakeRequestDecision::RateNotProfitable);
    }

    #[test]
    fn no_new_sell_order_if_no_btc_balance_change() {
        let mut maker = Maker {
            btc_balance: some_btc(1.0),
            ..StaticStub::static_stub()
        };

        let result = maker.update_bitcoin_balance(btc(1.0)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn no_new_buy_order_if_no_dai_balance_change() {
        let mut maker = Maker {
            dai_balance: some_dai(1.0),
            ..StaticStub::static_stub()
        };

        let result = maker.update_dai_balance(dai(1.0)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn new_orders_if_btc_balance_change() {
        let mut maker = Maker {
            btc_balance: some_btc(1.0),
            dai_balance: some_dai(1.0),
            fixed_rate: 1.0.try_into().unwrap(),
            ..StaticStub::static_stub()
        };
        let new_balance = btc(0.5);

        maker
            .update_bitcoin_balance(new_balance)
            .unwrap()
            .expect("to publish new orders if btc balance changes");

        assert_eq!(maker.btc_balance, Some(new_balance))
    }

    #[test]
    fn new_orders_if_dai_balance_change() {
        let mut maker = Maker {
            btc_balance: some_btc(1.0),
            dai_balance: some_dai(1.0),
            fixed_rate: 1.0.try_into().unwrap(),
            ..StaticStub::static_stub()
        };
        let new_balance = dai(0.5);

        maker
            .update_dai_balance(new_balance)
            .unwrap()
            .expect("to publish new orders if dai balance changes");

        assert_eq!(maker.dai_balance, Some(new_balance))
    }

    #[test]
    fn published_sell_order_can_be_taken() {
        let strategy = strategy::AllIn::new(
            StaticStub::static_stub(),
            None,
            Some(btc(1.0)),
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let mut maker = Maker {
            btc_balance: some_btc(3.0),
            fixed_rate: 1.0.try_into().unwrap(),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_sell_order = maker.new_sell_order().unwrap();
        assert_eq!(new_sell_order.quantity.sats(), btc(1.0).as_sat());

        let result = maker.process_taken_order(new_sell_order).unwrap();

        assert_eq!(result, TakeRequestDecision::GoForSwap);
    }

    #[test]
    fn published_buy_order_can_be_taken() {
        let strategy = strategy::AllIn::new(
            StaticStub::static_stub(),
            Some(btc(1.0)),
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let mut maker = Maker {
            dai_balance: some_dai(3.0),
            fixed_rate: 1.0.try_into().unwrap(),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_buy_order = maker.new_buy_order().unwrap();
        assert_eq!(
            wbtc::Amount::try_from(new_buy_order.quote()).unwrap(),
            dai(1.0)
        );

        let result = maker.process_taken_order(new_buy_order).unwrap();

        assert_eq!(result, TakeRequestDecision::GoForSwap);
    }

    #[test]
    fn new_buy_order_with_max_buy() {
        let strategy = strategy::AllIn::new(
            StaticStub::static_stub(),
            Some(btc(0.002)),
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let maker = Maker {
            dai_balance: some_dai(20.0),
            fixed_rate: 9000.0.try_into().unwrap(),
            btc_balance: some_btc(1.0),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_buy_order = maker.new_buy_order().unwrap();
        assert_eq!(new_buy_order.quantity.to_inner(), btc(0.002));

        assert_eq!(
            wbtc::Amount::try_from(new_buy_order.quote()).unwrap(),
            dai(18.0)
        );
    }

    #[test]
    fn new_buy_order() {
        let strategy = strategy::AllIn::new(
            StaticStub::static_stub(),
            None,
            None,
            Spread::static_stub(),
            StaticStub::static_stub(),
        );

        let maker = Maker {
            dai_balance: some_dai(20.0),
            fixed_rate: 10000.0.try_into().unwrap(),
            btc_balance: some_btc(1.0),
            strategy,
            ..StaticStub::static_stub()
        };

        let new_buy_order = maker.new_buy_order().unwrap();
        assert_eq!(new_buy_order.quantity.to_inner(), btc(0.002));
        assert_eq!(
            wbtc::Amount::try_from(new_buy_order.quote()).unwrap(),
            dai(20.0)
        );
    }
}

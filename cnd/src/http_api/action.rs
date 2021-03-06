use crate::{asset, bitcoin_fees::BitcoinFees, ethereum::ChainId, identity};
use anyhow::Result;
use comit::{
    ethereum::UnformattedData,
    ledger,
    swap::{
        actions::{CallContract, DeployContract, SendToAddress},
        Action,
    },
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type", content = "payload")]
pub enum ActionResponseBody {
    BitcoinSendAmountToAddress {
        to: bitcoin::Address,
        amount: String,
        network: ledger::Bitcoin,
    },
    BitcoinBroadcastSignedTransaction {
        hex: String,
        network: ledger::Bitcoin,
    },
    EthereumDeployContract {
        data: crate::ethereum::UnformattedData,
        amount: asset::Ether,
        gas_limit: crate::ethereum::U256,
        chain_id: ChainId,
    },
    EthereumCallContract {
        contract_address: identity::Ethereum,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<crate::ethereum::UnformattedData>,
        gas_limit: crate::ethereum::U256,
        chain_id: ChainId,
    },
}

impl From<SendToAddress> for ActionResponseBody {
    fn from(action: SendToAddress) -> Self {
        let SendToAddress {
            to,
            amount,
            network,
        } = action;
        ActionResponseBody::BitcoinSendAmountToAddress {
            to,
            amount: amount.as_sat().to_string(),
            network,
        }
    }
}

impl From<DeployContract> for ActionResponseBody {
    fn from(action: DeployContract) -> Self {
        let DeployContract {
            amount,
            chain_id,
            gas_limit,
            data,
        } = action;

        ActionResponseBody::EthereumDeployContract {
            data: UnformattedData(data),
            amount,
            gas_limit: gas_limit.into(),
            chain_id,
        }
    }
}

impl From<CallContract> for ActionResponseBody {
    fn from(action: CallContract) -> Self {
        let CallContract {
            to,
            data,
            gas_limit,
            chain_id,
        } = action;

        ActionResponseBody::EthereumCallContract {
            contract_address: to,
            data: data.map(UnformattedData),
            gas_limit: gas_limit.into(),
            chain_id,
        }
    }
}

impl From<comit::Never> for ActionResponseBody {
    fn from(_: comit::Never) -> Self {
        unreachable!("impl should be removed once ! type is stabilised")
    }
}

impl ActionResponseBody {
    pub async fn from_action(
        action: comit::swap::Action,
        bitcoin_fees: BitcoinFees,
    ) -> Result<Self> {
        Ok(match action {
            Action::Herc20Deploy(inner) => inner.into(),
            Action::Herc20Fund(inner) => inner.into(),
            Action::Herc20Redeem(inner, _) => inner.into(),
            Action::HbitFund(inner) => inner.into(),
            Action::HbitRedeem(inner, _) => {
                let network = inner.network;
                let rate = bitcoin_fees.get_per_vbyte_rate().await?;

                let transaction = inner.sign(&crate::SECP, rate)?;
                Self::BitcoinBroadcastSignedTransaction {
                    hex: hex::encode(bitcoin::consensus::serialize(&transaction)),
                    network,
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        asset::ethereum::FromWei, bitcoin::Address as BitcoinAddress, ethereum::U256, identity,
    };
    use comit::ethereum::UnformattedData;
    use std::str::FromStr;

    #[test]
    fn call_contract_serializes_correctly_to_json_with_none() {
        let addr =
            identity::Ethereum::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap();
        let chain_id = ChainId::from(3);
        let contract = ActionResponseBody::EthereumCallContract {
            contract_address: addr,
            data: None,
            gas_limit: U256::from(1),
            chain_id,
        };
        let serialized = serde_json::to_string(&contract).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"ethereum-call-contract","payload":{"contract_address":"0x0a81e8be41b21f651a71aab1a85c6813b8bbccf8","gas_limit":"0x1","chain_id":3}}"#
        );
    }

    #[test]
    fn deploy_contract_serializes_correctly_to_json() {
        let response_body = ActionResponseBody::EthereumDeployContract {
            data: UnformattedData(vec![0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0x10]),
            amount: asset::Ether::from_wei(10000u32),
            gas_limit: U256::from(1),
            chain_id: ChainId::from(3),
        };

        let serialized = serde_json::to_string(&response_body).unwrap();

        assert_eq!(
            serialized,
            r#"{"type":"ethereum-deploy-contract","payload":{"data":"0x01020304050607080910","amount":"10000","gas_limit":"0x1","chain_id":3}}"#
        );
    }

    #[test]
    fn bitcoin_send_amount_to_address_serializes_correctly_to_json() {
        let to = BitcoinAddress::from_str("2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9").unwrap();
        let amount = asset::Bitcoin::from_sat(100_000_000);

        let input = &[
            ActionResponseBody::from(SendToAddress {
                to: to.clone(),
                amount,
                network: ledger::Bitcoin::Mainnet,
            }),
            ActionResponseBody::from(SendToAddress {
                to: to.clone(),
                amount,
                network: ledger::Bitcoin::Testnet,
            }),
            ActionResponseBody::from(SendToAddress {
                to,
                amount,
                network: ledger::Bitcoin::Regtest,
            }),
        ];

        let expected = &[
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"mainnet"}}"#,
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"testnet"}}"#,
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"regtest"}}"#,
        ];

        let actual = input
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<String>, serde_json::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}

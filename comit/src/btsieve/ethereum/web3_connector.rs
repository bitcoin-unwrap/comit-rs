use crate::{
    btsieve::{
        ethereum::{Event, GetLogs, ReceiptByHash, TransactionByHash},
        jsonrpc, BlockByHash, ConnectedNetwork, LatestBlock,
    },
    ethereum::{ChainId, Hash, Log, Transaction, TransactionReceipt},
};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug)]
pub struct Web3Connector {
    client: jsonrpc::Client,
}

impl Web3Connector {
    pub fn new(node_url: reqwest::Url) -> Self {
        Self {
            client: jsonrpc::Client::new(node_url),
        }
    }

    pub async fn net_version(&self) -> Result<ChainId> {
        let version = self
            .client
            .send::<Vec<()>, String>(jsonrpc::Request::new("net_version", vec![]))
            .await?;

        Ok(ChainId::from(version.parse::<u32>()?))
    }
}

#[async_trait]
impl LatestBlock for Web3Connector {
    type Block = crate::ethereum::Block;

    async fn latest_block(&self) -> Result<Self::Block> {
        let block: Self::Block = self
            .client
            .send(jsonrpc::Request::new("eth_getBlockByNumber", vec![
                jsonrpc::serialize("latest")?,
                jsonrpc::serialize(true)?,
            ]))
            .await?;

        Ok(block)
    }
}

#[async_trait]
impl BlockByHash for Web3Connector {
    type Block = crate::ethereum::Block;
    type BlockHash = crate::ethereum::Hash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block> {
        let block = self
            .client
            .send(jsonrpc::Request::new("eth_getBlockByHash", vec![
                jsonrpc::serialize(&block_hash)?,
                jsonrpc::serialize(true)?,
            ]))
            .await?;

        Ok(block)
    }
}

#[async_trait]
impl ReceiptByHash for Web3Connector {
    async fn receipt_by_hash(&self, transaction_hash: Hash) -> Result<TransactionReceipt> {
        let receipt = self
            .client
            .send(jsonrpc::Request::new("eth_getTransactionReceipt", vec![
                jsonrpc::serialize(transaction_hash)?,
            ]))
            .await?;

        Ok(receipt)
    }
}

#[async_trait]
impl TransactionByHash for Web3Connector {
    async fn transaction_by_hash(&self, transaction_hash: Hash) -> Result<Transaction> {
        let transaction = self
            .client
            .send(jsonrpc::Request::new("eth_getTransactionByHash", vec![
                jsonrpc::serialize(transaction_hash)?,
            ]))
            .await?;

        Ok(transaction)
    }
}

#[async_trait]
impl ConnectedNetwork for Web3Connector {
    type Network = ChainId;

    async fn connected_network(&self) -> anyhow::Result<ChainId> {
        let chain_id = self.net_version().await?;

        Ok(chain_id)
    }
}

#[async_trait]
impl GetLogs for Web3Connector {
    async fn get_logs(&self, event: Event) -> Result<Vec<Log>> {
        // Ideally, we would be computing this based on start-of-swap.
        // However, some manual testing on mainnet shows that even specifying 0 is
        // reasonably performant (~ 1sec for Infura). Hence, we just specify 0
        // here to be on the safe side.
        let from_block = "0x0";

        let logs = self
            .client
            .send(jsonrpc::Request::new("eth_getLogs", vec![
                serde_json::json!({
                    "fromBlock": from_block,
                    "address": event.address,
                    "topics": event.topics
                }),
            ]))
            .await?;

        Ok(logs)
    }
}

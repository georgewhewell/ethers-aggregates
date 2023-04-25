use ethers::abi::RawLog;
use ethers::contract::Contract;
use ethers::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::ops::Deref;
use tokio::io::AsyncWriteExt;

mod error;
pub use error::AggregateError;

#[derive(Serialize, Deserialize, Default)]
pub struct CachedAggregate<T: State> {
    pub address: Address,
    pub chain: Chain,
    pub block_number: u64,
    filter: Filter,
    #[serde(deserialize_with = "T::deserialize")]
    pub state: T,
}

impl<T: State> CachedAggregate<T> {
    fn filename(&self) -> String {
        format!("{}-{}.json", self.chain as u64, self.address)
    }

    /// Create a new aggregate from a contract
    pub async fn sync<M: Middleware>(
        &mut self,
        client: impl AsRef<M>,
    ) -> Result<(), AggregateError> {
        let latest_block = client
            .as_ref()
            .get_block_number()
            .await
            .map_err(|e| AggregateError::Error(format!("Error getting latest block: {}", e)))?;
        tracing::info!("latest block: {}", latest_block);
        if self.block_number < latest_block.as_u64() {
            tracing::info!("fetching from {} to {}", self.block_number, latest_block);
            let filter = &self.filter.clone().from_block(self.block_number);
            let mut stream = client.as_ref().get_logs_paginated(filter, 10_000);

            let mut last_write = std::time::Instant::now();
            while let Some(log) = stream.next().await {
                match log {
                    Ok(log) => {
                        tracing::info!("got log: {:#?}", log);
                        if let Ok(event) = T::Events::decode_log(&RawLog {
                            topics: log.topics,
                            data: log.data.to_vec(),
                        }) {
                            self.state.handle(event)?;
                            if last_write.elapsed() > std::time::Duration::from_secs(60) {
                                self.snapshot(&self.filename()).await?;
                                last_write = std::time::Instant::now();
                            }
                        }
                        // todo: only update block number once we get all the logs
                        self.block_number = log.block_number.unwrap().as_u64();
                    }
                    Err(e) => {
                        return Err(AggregateError::Error(format!("Error getting logs: {}", e)));
                    }
                }
            }
        }
        self.snapshot(&self.filename()).await?;
        Ok(())
    }

    /// Save the current state to disk
    pub async fn snapshot(&self, filename: &str) -> Result<(), AggregateError> {
        let filename_tmp = format!("{}.tmp", filename);
        let file = tokio::fs::File::create(&filename_tmp).await?;
        let mut writer = tokio::io::BufWriter::new(file);
        let json = serde_json::to_string_pretty(&self)?;
        writer.write_all(json.as_bytes()).await?;
        std::fs::rename(filename_tmp, filename)?;
        Ok(())
    }
}

pub trait State: Serialize + Default + DeserializeOwned {
    type Events: EthLogDecode;

    fn from_contract<M: Middleware>(
        contract: impl Deref<Target = Contract<M>>,
        chain: Chain,
        block_number: u64,
    ) -> Result<CachedAggregate<Self>, AggregateError>
    where
        Self: Sized,
    {
        let address = contract.address();
        let filename = format!("{}-{}.json", chain as u64, address);
        if let Ok(contents) = std::fs::read_to_string(filename) {
            Ok(serde_json::from_str(&contents)?)
        } else {
            let filter = Filter::default()
                .address(address)
                .topic0(ValueOrArray::Array(
                    contract.as_ref().events().map(|e| e.signature()).collect(),
                ));
            Ok(CachedAggregate {
                address,
                chain,
                block_number,
                state: Self::default(),
                filter,
            })
        }
    }

    fn handle(&mut self, event: Self::Events) -> Result<(), AggregateError>;
}

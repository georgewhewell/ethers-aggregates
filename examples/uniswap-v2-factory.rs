use ethers::prelude::*;
use ethers_aggregates::{AggregateError, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Generate bindings for contract
abigen!(
    UniswapV2FactoryContract,
    r#"[
        event PairCreated(address indexed token0, address indexed token1, address pair, uint)
    ]"#,
);

// Define some struct to store the event
#[derive(Serialize, Deserialize, Default)]
struct UniswapV2Pool {
    token_0: Address,
    token_1: Address,
    pair: Address,
}

// Define some struct representing the aggregation
#[derive(Serialize, Deserialize, Default)]
struct UniswapV2Factory {
    num_pools: u32,
    pools: Vec<UniswapV2Pool>,
}

// Implement the State trait
impl State for UniswapV2Factory {
    // Event type from the contract bindings
    type Events = UniswapV2FactoryContractEvents;

    // Accumulate state from events
    fn handle(&mut self, event: Self::Events) -> Result<(), AggregateError> {
        match event {
            UniswapV2FactoryContractEvents::PairCreatedFilter(PairCreatedFilter {
                token_0,
                token_1,
                pair,
                ..
            }) => {
                self.num_pools += 1;
                self.pools.push(UniswapV2Pool {
                    token_0,
                    token_1,
                    pair,
                });
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let client = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
    let contract = UniswapV2FactoryContract::new(
        "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
            .parse::<Address>()
            .unwrap(),
        client.clone(),
    );
    let mut state = UniswapV2Factory::from_contract(contract, Chain::Mainnet, 10000835).unwrap();

    println!("starting sync..");

    state.sync(&client).await.unwrap();
}

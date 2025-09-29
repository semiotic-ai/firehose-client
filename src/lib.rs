// Copyright 2024-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # Rust Firehose Client
//!
//! Rust implementation of a client for the [StreamingFast Firehose](https://firehose.streamingfast.io/)
//! gRPC Fetch `Block` and Stream `Block`s APIs.
//!
//! ## Fetching an Ethereum Block
//!
//! ```rust,ignore
//! use firehose_client::{Chain, FirehoseClient};
//! use vee::EthBlock as Block;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = FirehoseClient::new(Chain::Ethereum);
//!
//!     if let Some(response) = client.fetch_block(20672593).await?.ok() {
//!         let block = Block::try_from(response.into_inner())?;
//!         assert_eq!(block.number, 20672593);
//!         assert_eq!(
//!             format!("0x{}", hex::encode(block.hash)).as_str(),
//!             "0xea48ba1c8e38ea586239e9c5ec62949ddd79404c6006c099bb02a8b22ddd18e4"
//!         );
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Streaming Ethereum Blocks
//!
//! ```rust,ignore
//! use firehose_client::{Chain, FirehoseClient};
//! use futures::StreamExt;
//! use vee::EthBlock as Block;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     const TOTAL_BLOCKS: u64 = 8192;
//!     const START_BLOCK: u64 = 19581798;
//!
//!     let mut client = FirehoseClient::new(Chain::Ethereum);
//!     let mut stream = client
//!         .stream_blocks::<Block>(START_BLOCK, TOTAL_BLOCKS)
//!         .await?;
//!
//!     while let Some(block) = stream.next().await {
//!         // Do Something with the extracted stream of blocks.
//!     }
//!     Ok(())
//! }
//! ```
//!

mod client;
mod error;
mod tls;

pub use crate::client::{Chain, FirehoseClient};

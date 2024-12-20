// Copyright 2024-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # Example: Stream Beacon Blocks
//!
//! Demonstrates how to stream a range of blocks from Firehose Beacon

use beacon_protos::Block as FirehoseBeaconBlock;
use firehose_client::{Chain, FirehoseClient};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    // Testing this so far without proper benchmarking, the time taken to fetch the blocks
    // grows linearly with the number of TOTAL_BLOCKS requested, to around 20 minutes for 8192 blocks!
    const TOTAL_SLOTS: u64 = 100;
    const START_SLOT: u64 = 9968872;

    let mut client = FirehoseClient::new(Chain::Beacon);
    let mut stream = client
        .stream_blocks::<FirehoseBeaconBlock>(START_SLOT, TOTAL_SLOTS)
        .await
        .unwrap();

    let mut blocks: Vec<FirehoseBeaconBlock> = Vec::with_capacity(TOTAL_SLOTS as usize);

    while let Some(block) = stream.next().await {
        blocks.push(block);
    }

    // For now, just using this to signal that the test has completed
    assert_eq!(blocks.len(), TOTAL_SLOTS as usize);

    println!("stream_beacon ran to completion!");
}

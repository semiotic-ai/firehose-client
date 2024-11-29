// Copyright 2024-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use crate::error::ClientError;
use dotenvy::{dotenv, var};
use firehose_rs::{
    FetchClient, FromResponse, HasNumberOrSlot, Request, SingleBlockRequest, SingleBlockResponse,
    StreamClient,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{
    metadata::MetadataValue,
    transport::{Channel, Uri},
    Response, Status,
};
use tracing::{error, info, trace};

/// Work with the fetch and streaming APIs supported by [StreamingFast Firehose](https://firehose.streamingfast.io/).
pub struct FirehoseClient {
    chain: Chain,
    fetch_client: Option<FetchClient<Channel>>,
    stream_client: Option<StreamClient<Channel>>,
}

impl FirehoseClient {
    pub fn new(chain: Chain) -> Self {
        Self {
            chain,
            fetch_client: None,
            stream_client: None,
        }
    }

    /// The inner [`Result`] is the Firehose response, which can be either a [`Response`] or a [`Status`],
    /// which is needed for handling empty slots on the Beacon chain.
    pub async fn fetch_block(
        &mut self,
        number: u64,
    ) -> Result<Result<Response<SingleBlockResponse>, Status>, ClientError> {
        if self.fetch_client.is_none() {
            self.fetch_client = Some(fetch_client(self.chain).await?);
        }
        let mut request = create_single_block_fetch_request(number);

        request.insert_api_key_if_provided(self.chain);

        info!("Requesting block number:\n\t{}", number);
        Ok(self.fetch_client.as_mut().unwrap().block(request).await)
    }

    /// The tonic docs encourage cloning the client.
    pub async fn get_streaming_client(&mut self) -> Result<StreamClient<Channel>, ClientError> {
        let client = if let Some(client) = self.stream_client.clone() {
            client
        } else {
            self.stream_client = Some(stream_client(self.chain).await?);
            self.stream_client.clone().unwrap()
        };
        Ok(client)
    }

    /// Stream blocks from the Firehose service.
    ///
    /// This generic method streams blocks of type `T` from a specified start point,
    /// for a given total number of blocks. It ensures that any missed slots or block
    /// numbers are filled in with the last valid block to maintain continuity in the stream.
    ///
    /// # Type Parameters
    ///
    /// * `T`: The block type, which must implement the [`FromResponse`] and
    ///   [`HasNumberOrSlot`] traits. The type must also be `Clone`, `Send`, and `Sync` to
    ///   support concurrent processing.
    ///
    /// # Parameters
    ///
    /// * `start`: The starting block number or slot to begin streaming from.
    /// * `total`: The total number of blocks or slots to stream.
    ///
    /// # Returns
    ///
    /// Returns a [`Result`] wrapping a stream (`futures::Stream`) of blocks of type `T`.
    /// The stream will yield blocks as they are received from the Firehose service.
    ///
    /// # Slot/Number Handling
    ///
    /// For Beacon chain blocks, missing slots are filled in with the last
    /// valid block. For verifying extracted blocks, we need block roots for each block.
    /// For missed slots the block root of the last non-missed slot block is used,
    /// as this is what gets added to the `block_roots` buffer on Beacon State.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// use firehose_client::{FirehoseClient, FirehoseBeaconBlock};
    /// use futures::StreamExt;
    ///
    /// let mut client = FirehoseClient::new(...);
    /// let stream = client
    ///     .stream_blocks::<FirehoseBeaconBlock>(start_slot, total_slots)
    ///     .await?;
    ///
    /// stream.for_each(|block| {
    ///     println!("Received block: {:?}", block);
    ///     futures::future::ready(())
    /// }).await;
    /// ```
    ///
    /// # Concurrency
    ///
    /// This method spawns a background task to handle the streaming and conversion
    /// logic, ensuring that the main thread is return a stream to process incoming blocks.
    ///
    /// *NOTE*: For Beacon chain blocks, missing slots are filled in with the last
    /// valid block. For verifying extracted blocks, we need block roots for each block.
    /// For missed slots the block root of the last non-missed slot block is used,
    /// as this is what gets added to the `block_roots` buffer on Beacon State.
    pub async fn stream_blocks<T>(
        &mut self,
        start: u64,
        total: u64,
    ) -> Result<impl futures::Stream<Item = T>, ClientError>
    where
        T: FromResponse + HasNumberOrSlot + Clone + Send + Sync + 'static,
    {
        let (tx, rx) = tokio::sync::mpsc::channel::<T>(8192);

        let chain = self.chain;
        let client = self.get_streaming_client().await?;

        tokio::spawn(async move {
            let mut blocks = 0;
            let mut last_valid_slot: Option<u64> = None;
            let mut last_valid_block: Option<T> = None;

            while blocks < total {
                let mut client = client.clone();
                let request = create_blocks_streaming_request(
                    chain,
                    start + blocks,
                    start + total - 1,
                    BlocksRequested::All,
                );

                match client.blocks(request).await {
                    Ok(response) => {
                        let mut stream_inner = response.into_inner();
                        while let Ok(Some(block_msg)) = stream_inner.message().await {
                            if blocks % 100 == 0 {
                                trace!("Blocks fetched: {}", blocks);
                            }

                            match T::from_response(block_msg) {
                                Ok(block) => {
                                    if let Some(last_slot) = last_valid_slot {
                                        let missed_slots =
                                            block.number_or_slot().saturating_sub(last_slot + 1);
                                        if missed_slots > 0 {
                                            trace!(
                                                "Detected {} missed slots, filling in...",
                                                missed_slots
                                            );
                                            if let Some(ref last_block) = last_valid_block {
                                                for _ in 0..missed_slots {
                                                    blocks += 1;
                                                    let _ = tx.send(last_block.clone()).await;
                                                }
                                            }
                                        }
                                    }

                                    if let Chain::Beacon = chain {
                                        last_valid_slot = Some(block.number_or_slot());
                                        last_valid_block = Some(block.clone());
                                    }

                                    blocks += 1;
                                    let _ = tx.send(block).await;
                                }
                                Err(e) => {
                                    error!("Failed to convert block message: {e}");
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to stream blocks: {:?}", e);
                        break;
                    }
                };
            }
        });

        Ok(ReceiverStream::new(rx))
    }
}

async fn build_and_connect_channel(uri: Uri) -> Result<Channel, tonic::transport::Error> {
    if uri.scheme_str() != Some("https") {
        return Channel::builder(uri).connect().await;
    }

    let config = crate::tls::config();

    Channel::builder(uri)
        .tls_config(config.clone())?
        .connect()
        .await
}

fn create_blocks_streaming_request(
    chain: Chain,
    start_block_num: u64,
    stop_block_num: u64,
    blocks_requested: BlocksRequested,
) -> tonic::Request<Request> {
    let mut request = tonic::Request::new(Request {
        start_block_num: start_block_num as i64,
        stop_block_num,
        final_blocks_only: blocks_requested.into(),
        ..Default::default()
    });
    request.insert_api_key_if_provided(chain);
    request
}

async fn fetch_client(firehose: Chain) -> Result<FetchClient<Channel>, ClientError> {
    Ok(FetchClient::new({
        let uri = firehose.uri_from_env()?;
        build_and_connect_channel(uri).await?
    }))
}

async fn stream_client(firehose: Chain) -> Result<StreamClient<Channel>, ClientError> {
    Ok(StreamClient::new({
        let uri = firehose.uri_from_env()?;
        build_and_connect_channel(uri).await?
    }))
}

pub enum BlocksRequested {
    All,
    FinalOnly,
}

impl From<BlocksRequested> for bool {
    fn from(blocks_requested: BlocksRequested) -> bool {
        match blocks_requested {
            BlocksRequested::All => false,
            BlocksRequested::FinalOnly => true,
        }
    }
}

/// Create a [`SingleBlockRequest`] for the given *number*.
/// Number is slot number for beacon blocks.
fn create_single_block_fetch_request(num: u64) -> tonic::Request<SingleBlockRequest> {
    tonic::Request::new(SingleBlockRequest::new(num))
}

trait FirehoseRequest {
    fn insert_api_key_if_provided(&mut self, endpoint: Chain);
}

impl<T> FirehoseRequest for tonic::Request<T> {
    fn insert_api_key_if_provided(&mut self, endpoint: Chain) {
        insert_api_key_if_provided(self, endpoint);
    }
}

fn insert_api_key_if_provided<T>(request: &mut tonic::Request<T>, chain: Chain) {
    if let Ok(api_key) = var(chain.api_key_env_var_as_str()) {
        let api_key_header = MetadataValue::from_str(&api_key).expect("Invalid API key format");
        request.metadata_mut().insert("x-api-key", api_key_header);
    }
}

/// Extract blocks with [`FirehoseClient`] from an extendable union of chain variants.
#[derive(Clone, Copy, Debug)]
pub enum Chain {
    Ethereum,
    Beacon,
    Arbitrum,
}

impl Chain {
    fn api_key_env_var_as_str(&self) -> &str {
        match self {
            Self::Beacon => "BEACON_API_KEY",
            Self::Ethereum => "ETHEREUM_API_KEY",
            Self::Arbitrum => "ARBITRUM_API_KEY",
        }
    }

    fn uri_from_env(&self) -> Result<Uri, ClientError> {
        dotenv()?;

        let (url, port) = match self {
            Self::Ethereum => (
                var("FIREHOSE_ETHEREUM_URL")?,
                var("FIREHOSE_ETHEREUM_PORT")?,
            ),
            Self::Beacon => (var("FIREHOSE_BEACON_URL")?, var("FIREHOSE_BEACON_PORT")?),
            Self::Arbitrum => (
                var("FIREHOSE_ARBITRUM_URL")?,
                var("FIREHOSE_ARBITRUM_PORT")?,
            ),
        };

        Ok(format!("{}:{}", url, port).parse::<Uri>()?)
    }
}

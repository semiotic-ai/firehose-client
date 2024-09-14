use std::str::FromStr;

use sf_protos::firehose::v2::{
    single_block_request::{BlockNumber, Reference},
    Request, SingleBlockRequest,
};

use crate::client::endpoint::Firehose;

pub enum BlocksRequested {
    All,
    FinalOnly,
}

/// Create a `SingleBlockRequest` for the given *number*.
/// Number is slot number for beacon blocks.
pub fn create_request(num: u64) -> tonic::Request<SingleBlockRequest> {
    tonic::Request::new(SingleBlockRequest {
        reference: Some(Reference::BlockNumber(BlockNumber { num })),
        ..Default::default()
    })
}

pub fn create_blocks_request(
    start_block_num: i64,
    stop_block_num: u64,
    blocks_requested: BlocksRequested,
) -> tonic::Request<Request> {
    use BlocksRequested::*;
    tonic::Request::new(Request {
        start_block_num,
        stop_block_num,
        final_blocks_only: match blocks_requested {
            All => false,
            FinalOnly => true,
        },
        ..Default::default()
    })
}

pub trait FirehoseRequest {
    fn insert_api_key_if_provided(&mut self, endpoint: Firehose);
}

impl FirehoseRequest for tonic::Request<SingleBlockRequest> {
    fn insert_api_key_if_provided(&mut self, endpoint: Firehose) {
        insert_api_key_if_provided(self, endpoint);
    }
}

impl FirehoseRequest for tonic::Request<Request> {
    fn insert_api_key_if_provided(&mut self, endpoint: Firehose) {
        insert_api_key_if_provided(self, endpoint);
    }
}

fn insert_api_key_if_provided<T>(request: &mut tonic::Request<T>, endpoint: Firehose) {
    use Firehose::*;
    let var = match endpoint {
        Ethereum => "ETHEREUM_API_KEY",
        Beacon => "BEACON_API_KEY",
    };
    if let Ok(api_key) = dotenvy::var(var) {
        let api_key_header =
            tonic::metadata::MetadataValue::from_str(&api_key).expect("Invalid API key format");
        request.metadata_mut().insert("x-api-key", api_key_header);
    }
}
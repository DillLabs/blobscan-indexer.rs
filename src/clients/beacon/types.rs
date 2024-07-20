use std::{fmt, str::FromStr};

use ethers::types::{Bytes, H256};
use serde::{Deserialize, Serialize};

use crate::slots_processor::BlockData;

#[derive(Serialize, Debug, Clone)]
pub enum BlockId {
    Head,
    Finalized,
    Slot(u32),
    Hash(H256),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Head,
    FinalizedCheckpoint,
    ChainReorg,
}

#[derive(Deserialize, Debug)]
pub struct ExecutionPayload {
    pub block_hash: H256,
    #[serde(deserialize_with = "deserialize_number")]
    pub block_number: u32,
}

#[derive(Deserialize, Debug)]
pub struct BlockBody {
    pub execution_payload: Option<ExecutionPayload>,
    pub blob_kzg_commitments: Option<Vec<String>>,
}
#[derive(Deserialize, Debug)]
pub struct BlockMessage {
    #[serde(deserialize_with = "deserialize_number")]
    pub slot: u32,
    pub proposer_index: u64,
    pub body: BlockBody,
    pub parent_root: H256,
}

#[derive(Deserialize, Debug)]
pub struct Block {
    pub message: BlockMessage,
}

#[derive(Deserialize, Debug)]
pub struct BlockResponse {
    pub data: Block,
}

#[derive(Deserialize, Debug)]
pub struct ProposersResponse {
    pub data: Vec<Proposer>,
}

#[derive(Deserialize, Debug)]
pub struct Proposer {
    pub pubkey: String,
    pub validator_index: String,
    #[serde(deserialize_with = "deserialize_number")]
    pub slot: u32,
}

#[derive(Deserialize, Debug)]
pub struct Blob {
    pub index: String,
    pub kzg_commitment: String,
    pub kzg_proof: String,
    pub blob: Bytes,
}

#[derive(Deserialize, Debug)]
pub struct BlobsResponse {
    pub data: Vec<Blob>,
}

#[derive(Deserialize, Debug)]
pub struct Column {
    pub index: String,
    pub blob_kzg_commitments: Vec<String>,
    pub segment_kzg_proofs: Vec<String>,
    pub segments: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct ColumnsResponse {
    pub data: Vec<Column>,
}

#[derive(Deserialize, Debug)]
pub struct BlockHeaderResponse {
    pub data: BlockHeader,
}

#[derive(Deserialize, Debug)]
pub struct BlockHeader {
    pub root: H256,
    pub header: InnerBlockHeader,
}
#[derive(Deserialize, Debug)]
pub struct InnerBlockHeader {
    pub message: BlockHeaderMessage,
}

#[derive(Deserialize, Debug)]
pub struct BlockHeaderMessage {
    pub parent_root: H256,
    #[serde(deserialize_with = "deserialize_number")]
    pub slot: u32,
}

#[derive(Deserialize, Debug)]
pub struct ChainReorgEventData {
    pub old_head_block: H256,
    pub new_head_block: H256,
    #[serde(deserialize_with = "deserialize_number")]
    pub slot: u32,
    #[serde(deserialize_with = "deserialize_number")]
    pub depth: u32,
}

#[derive(Deserialize, Debug)]
pub struct HeadEventData {
    #[serde(deserialize_with = "deserialize_number")]
    pub slot: u32,
    pub block: H256,
}

#[derive(Deserialize, Debug)]
pub struct FinalizedCheckpointEventData {
    pub block: H256,
}

fn deserialize_number<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;

    value.parse::<u32>().map_err(serde::de::Error::custom)
}

impl BlockId {
    pub fn to_detailed_string(&self) -> String {
        match self {
            BlockId::Head => String::from("head"),
            BlockId::Finalized => String::from("finalized"),
            BlockId::Slot(slot) => slot.to_string(),
            BlockId::Hash(hash) => format!("0x{:x}", hash),
        }
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockId::Head => write!(f, "head"),
            BlockId::Finalized => write!(f, "finalized"),
            BlockId::Slot(slot) => write!(f, "{}", slot),
            BlockId::Hash(hash) => write!(f, "{}", hash),
        }
    }
}

impl FromStr for BlockId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "head" => Ok(BlockId::Head),
            "finalized" => Ok(BlockId::Finalized),
            _ => match s.parse::<u32>() {
                Ok(num) => Ok(BlockId::Slot(num)),
                Err(_) => {
                    if s.starts_with("0x") {
                        match H256::from_str(s) {
                            Ok(hash) => Ok(BlockId::Hash(hash)),
                            Err(_) => Err(format!("Invalid block ID hash: {s}")),
                        }
                    } else {
                        Err(
                            format!("Invalid block ID: {s}. Expected 'head', 'finalized', a hash or a number."),
                        )
                    }
                }
            },
        }
    }
}

impl From<&Topic> for String {
    fn from(value: &Topic) -> Self {
        match value {
            Topic::ChainReorg => String::from("chain_reorg"),
            Topic::Head => String::from("head"),
            Topic::FinalizedCheckpoint => String::from("finalized_checkpoint"),
        }
    }
}

impl From<HeadEventData> for BlockData {
    fn from(event_data: HeadEventData) -> Self {
        Self {
            root: event_data.block,
            slot: event_data.slot,
        }
    }
}

impl From<ColumnsResponse> for BlobsResponse {
    fn from(columns_res: ColumnsResponse) -> Self {
        let mut blobs = Vec::new();
        let kzg_commitments = columns_res.data[0].blob_kzg_commitments.clone();
        
        //每个blob的index对应columns_res.data的每个column的index
        //第i个blob的kzg_commitment对应columns_res.data的第0号column的blob_kzg_commitments的第i个元素
        //每个blob的kzg_proof为空字符串
        //每个blob的kzg_proof为空bytes
        for (i, comm) in kzg_commitments.iter().enumerate() {
            blobs.push(Blob {
                index: i.to_string(),
                kzg_commitment: comm.clone(),
                kzg_proof: String::new(),
                blob: Bytes::from_str(comm).unwrap(),
            });
        }
        Self { data: blobs }
    }
}

impl From<Vec<String>> for BlobsResponse {
    fn from(kzg_commitments: Vec<String>) -> Self {
        let mut blobs = Vec::new();
        for (i, comm) in kzg_commitments.iter().enumerate() {
            blobs.push(Blob {
                index: i.to_string(),
                kzg_commitment: comm.clone(),
                kzg_proof: String::new(),
                blob: Bytes::from_str(comm).unwrap(),
            });
        }
        Self { data: blobs }
    }
}

use marine_rs_sdk::marine;
use serde::{ Deserialize, Serialize };
use serde_json::Value ;
use std::time::{ SystemTime, UNIX_EPOCH }; 

#[marine]
pub struct MetaContractResult {
    pub result: bool,
    pub metadatas: Vec<FinalMetadata>,
    pub error_string: String,
}

#[marine]
pub struct FinalMetadata {
    pub public_key: String,
    pub alias: String,
    pub content: String,
    pub loose: i64,
    pub version: String,
}

#[marine]
#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    pub hash: String,
    pub token_key: String,
    pub data_key: String,
    pub meta_contract_id: String,
    pub token_id: String,
    pub alias: String,
    pub cid: String,
    pub public_key: String,
    pub version: String,
    pub loose: i64,
}

#[marine]
#[derive(Debug, Clone)]
pub struct Transaction {
    pub hash: String,
    pub method: String,
    pub meta_contract_id: String,
    pub data_key: String,
    pub token_key: String,
    pub data: String,
    pub public_key: String,
    pub alias: String,
    pub timestamp: u64,
    pub chain_id: String,
    pub token_address: String,
    pub token_id: String,
    pub version: String,
    pub status: i64,
    pub mcdata: String,
}

#[marine]
#[derive(Debug, Default, Clone)]
pub struct MetaContract {
    pub hash: String,
    pub token_key: String,
    pub meta_contract_id: String,
    pub public_key: String,
    pub cid: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct SerdeMetadata {
    pub cid: String,
    pub mentionable: Option<bool>,
    pub owner: String
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub timestamp: u64,
    pub content: Value,
    pub previous: Value,
    pub transaction: Value,
}


#[derive(Debug, Deserialize, Serialize)]
pub struct FinalMention {
   pub timestamp: u64,
   pub mentionable: bool,
   pub owner: String
}

impl FinalMention {
   pub fn new(mentionable: Option<bool>, owner: String) -> Self {
        let now = SystemTime::now();
        let timestamp= now.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;
        let mentionable=  if let Some(false) = mentionable {false} else {true};

        FinalMention {
            timestamp,
            mentionable,
            owner 
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FilterQuery{
  pub column: String,
  pub op: String,
  pub query: String,
}

#[derive(Debug, Serialize)]
pub struct FilterOrdering {
  pub column: String,
  pub sort: String,
}

#[derive(Debug, Serialize)]
pub struct JSONRPCFilter {
  pub query: Vec<FilterQuery>,
  pub ordering: Vec<Option<FilterOrdering>>,
  pub from: u32,
  pub to: u32 
}

#[derive(Debug, Serialize)]
pub struct JSONRPCBody {
    pub jsonrpc: String,
    pub method: String,
    pub params: JSONRPCFilter,
    pub id: String,
}


#[derive(Debug, Deserialize)]
pub struct FdbMetadatasResult {
    pub success: bool,
    pub err_msg: String,
    pub metadatas: Vec<Metadata>
}

#[derive(Debug, Deserialize)]
pub struct JSONRPCResult{
    pub jsonrpc: String,
    pub method: String,
    pub result: FdbMetadatasResult
}

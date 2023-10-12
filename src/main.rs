#![allow(improper_ctypes)]

mod types;
mod data;
mod defaults;

use std::collections::HashMap;
use data::DataStructFork;
use defaults::{ DEFAULT_IPFS_MULTIADDR, DEFAULT_TIMEOUT_SEC, DEFAULT_LINEAGE_NODE_URL};
use marine_rs_sdk::marine;
use marine_rs_sdk::module_manifest;
use marine_rs_sdk::MountedBinaryResult;
use marine_rs_sdk::WasmLoggerBuilder;
use types::Block;
use types::MetaContract;
use types::Metadata;
use types::Transaction;
use types::{ SerdeMetadata, FinalMetadata, MetaContractResult, FinalMention };
use types::{ JSONRPCFilter, FilterQuery, JSONRPCBody, JSONRPCResult };
// use reqwest::Error;

module_manifest!();

pub fn main() {
    WasmLoggerBuilder::new()
        .with_log_level(log::LevelFilter::Info)
        .build()
        .unwrap();
}

#[marine]
pub fn on_execute(
    contract: MetaContract,
    metadatas: Vec<Metadata>,
    transaction: Transaction,
) -> MetaContractResult {
    let mut finals: Vec<FinalMetadata> = vec![];
    let final_mention: FinalMention;
    let mut origin_cid = "".to_string();
    let mut cid = "".to_string();
    let mut content: HashMap<String, FinalMention> = HashMap::new();

    let serde_metadata: Result<SerdeMetadata, serde_json::Error> = serde_json::from_str(&transaction.data.clone());

    match serde_metadata {
      Ok(tx_data) => {

        if tx_data.cid.is_empty() { 
          return MetaContractResult {
            result: false,
            metadatas: Vec::new(),
            error_string: "cid cannot be empty.".to_string(),
         };
        }

        if tx_data.owner.is_empty() { 
          return MetaContractResult {
            result: false,
            metadatas: Vec::new(),
            error_string: "owner cannot be empty.".to_string(),
         };
        }

        origin_cid = tx_data.cid;
        final_mention= FinalMention::new(tx_data.mentionable, tx_data.owner);

        for metadata in metadatas.clone(){
          if metadata.version == transaction.data_key {
            cid = metadata.cid;
          }
        }
      }
      Err(_) => {
        return MetaContractResult {
          result: false,
          metadatas: Vec::new(),
          error_string: "Data does not follow the required JSON schema".to_string(),
        }
      }
    }

    if !cid.is_empty() {
      let ipfs_get_result = get(cid.clone(), "".to_string(), 0);
      let block: Block = serde_json::from_str(&ipfs_get_result).unwrap();
      let deserialized_content: Result<HashMap<String, FinalMention>, serde_json::Error> = serde_json::from_value(block.content);

      match deserialized_content {
        Ok (mentions) => {
          content = mentions;

          let exists = content.get(&origin_cid);

          match exists {
            Some(mention) => {
              if transaction.public_key == mention.owner {
                content.insert(origin_cid, final_mention);
              }

              // if transaction.public_key == nft_owner {
              // content.insert(origin_cid, final_mention);
              // }
            }
            None => { content.insert(origin_cid, final_mention); }              
          }
        }
        Err(_)=> {}
      }
    } else {
      content.insert(origin_cid, final_mention);
    }

    let serialized_content= serde_json::to_string(&content);

    match serialized_content{
      Ok(content) => {

        finals.push(FinalMetadata {
            public_key: transaction.meta_contract_id,
            alias: "mentions".to_string(),
            content,
            version: transaction.data_key,
            loose: 0,
        });

        MetaContractResult {
            result: true,
            metadatas: finals,
            error_string: "".to_string(),
        }
      }
      Err(_) => {
        return MetaContractResult {
          result: false,
          metadatas: Vec::new(),
          error_string: "Unable to serialize content".to_string(),
        }
      }
    }
}

#[marine]
pub fn on_clone() -> bool {
    return false;
}

#[marine]
pub fn on_mint(
    contract: MetaContract,
    data_key: String,
    token_id: String,
    data: String,
) -> MetaContractResult {
    MetaContractResult {
        result: false,
        metadatas: vec![],
        error_string: "on_mint is not available".to_string(),
    }
}
/**
 * Get data from ipfs
 */
fn get(hash: String, api_multiaddr: String, timeout_sec: u64) -> String {
  let address: String;
  let t;

  if api_multiaddr.is_empty() {
      address = DEFAULT_IPFS_MULTIADDR.to_string();
  } else {
      address = api_multiaddr;
  }

  if timeout_sec == 0 {
      t = DEFAULT_TIMEOUT_SEC;
  } else {
      t = timeout_sec;
  }

  let args = vec![String::from("dag"), String::from("get"), hash];

  let cmd = make_cmd_args(args, address, t);

  let result = ipfs(cmd);

  String::from_utf8(result.stdout).unwrap()
}

pub fn make_cmd_args(args: Vec<String>, api_multiaddr: String, timeout_sec: u64) -> Vec<String> {
  args.into_iter()
      .chain(vec![
          String::from("--timeout"),
          get_timeout_string(timeout_sec),
          String::from("--api"),
          api_multiaddr,
      ])
      .collect()
}

#[inline]
pub fn get_timeout_string(timeout: u64) -> String {
  format!("{}s", timeout)
}

// Service
// - curl

#[marine]
#[link(wasm_import_module = "host")]
extern "C" {
  pub fn ipfs(cmd: Vec<String>) -> MountedBinaryResult;
}

/**
 * For now leaving it empty. Freedom of speech
 */
pub fn is_profane(text: &str) -> bool {
  let profane_words = vec!["", ""];
  profane_words.iter().any(|&word| {
    if word != "" {
      return text.contains(word)
    }
    false
  })
}

pub fn is_nft_storage_link(link: &str) -> bool {
  link == "" || link.starts_with("https://nftstorage.link/ipfs/")
}

/* async fn get_nft_owner(data_key:String) -> Result<String, Error> {
  let client = reqwest::Client::new();

  let query = [
    FilterQuery {
      column: "data_key".to_string(),
      op: "=".to_string(),
      query: data_key
    },
    FilterQuery {
      column: "meta_contract_id".to_string(),
      op: "=".to_string(),
      query: "0x01".to_string()
    }
  ].to_vec();

  let params = JSONRPCFilter {
    query,
    ordering: Vec::new(),
    from: 0,
    to: 0
  };

  let body = JSONRPCBody {
    jsonrpc: "2.0".to_string(),
    method: "search_metadatas".to_string(),
    params,
    id: "1".to_string()
  };

  let serialized_body = serde_json::to_string(&body).unwrap();

  let response = client.post(DEFAULT_LINEAGE_NODE_URL).body(serialized_body).send().await?;
  let metadata = response.json::<JSONRPCResult>().await?;

  let nft = metadata.result.metadatas.into_iter().nth(0);
  let owner_pk = if let Some(nft) = nft { nft.public_key } else { "".to_string() };

  Ok(owner_pk)
} */
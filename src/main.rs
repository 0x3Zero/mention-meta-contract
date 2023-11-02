#![allow(improper_ctypes)]

mod data;
mod defaults;
mod types;

use data::DataStructFork;
use defaults::{DEFAULT_IPFS_MULTIADDR, DEFAULT_LINEAGE_NODE_URL, DEFAULT_TIMEOUT_SEC};
use marine_rs_sdk::marine;
use marine_rs_sdk::module_manifest;
use marine_rs_sdk::MountedBinaryResult;
use marine_rs_sdk::WasmLoggerBuilder;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::string;
use types::Block;
use types::MetaContract;
use types::Metadata;
use types::Transaction;
use types::{FilterQuery, JSONRPCBody, JSONRPCFilter, JSONRPCResult};
use types::{FinalMention, FinalMetadata, MetaContractResult, SerdeMetadata};

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
    let parent_cid: String;
    let mut existing_mention: Metadata = Metadata::new();
    let mut content: FinalMention;

    let serde_metadata: Result<SerdeMetadata, serde_json::Error> =
        serde_json::from_str(&transaction.data.clone());

    match serde_metadata {
        Ok(tx_data) => {
            if tx_data.cid.is_empty() {
                return MetaContractResult {
                    result: false,
                    metadatas: Vec::new(),
                    error_string: "cid cannot be empty.".to_string(),
                };
            }

            parent_cid = tx_data.cid.clone();
            final_mention = FinalMention::new(tx_data.mentionable);

            for metadata in metadatas.clone() {
                if metadata.version == tx_data.cid
                    && metadata.data_key == transaction.data_key.clone()
                {
                    existing_mention = metadata;
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

    if !existing_mention.cid.is_empty() {
        let ipfs_get_result = get(existing_mention.cid.clone(), "".to_string(), 0);
        let block: Block = serde_json::from_str(&ipfs_get_result).unwrap();
        let deserialized_content: Result<FinalMention, serde_json::Error> =
            serde_json::from_value(block.content);

        match deserialized_content {
            Ok(mention) => {
                content = mention;

                if transaction.public_key == existing_mention.public_key {
                    content = final_mention;
                } else {
                    // ownerOf NFT able to disable mention here
                    // Check ownerOf NFT from smart contract
                    // TODO
                    return MetaContractResult {
                        result: false,
                        metadatas: Vec::new(),
                        error_string: "Not owner of the post".to_string(),
                    };
                }
            }
            Err(_) => {
                return MetaContractResult {
                    result: false,
                    metadatas: Vec::new(),
                    error_string: "Unable to deserialize ipfs content".to_string(),
                }
            }
        }
    } else {
        content = final_mention;
    }

    let serialized_content = serde_json::to_string(&content);

    match serialized_content {
        Ok(content) => {
            // TEMPORARY FIX
            // For now, there's no way for 0x01 to request block 0x01 to update NFT metadata.
            // For now, check if there's token, lineage_key data stored in 0x01 block. If not, add new block under 0x02
            let exists_token = metadatas.iter().any(|m| {
                m.public_key == "0x01" && m.alias == "token" && m.meta_contract_id == "0x01"
            });

            let exists_lineage_key = metadatas.iter().any(|m| {
                m.public_key == "0x01" && m.alias == "lineage_key" && m.meta_contract_id == "0x01"
            });

            if !exists_token {
                let content_1 = format!(
                    r#"{{ 
                        "address": "{}", 
                        "chain": "{}", 
                        "id": "{}"
                    }}"#,
                    transaction.token_address, transaction.chain_id, transaction.token_id
                );

                finals.push(FinalMetadata {
                    public_key: "0x01".to_string(),
                    alias: "token".to_string(),
                    content: content_1,
                    loose: 0,
                    version: "".to_string(),
                });
            }

            if !exists_lineage_key {
                finals.push(FinalMetadata {
                    public_key: "0x01".to_string(),
                    alias: "lineage_key".to_string(),
                    content: transaction.data_key.clone(),
                    loose: 0,
                    version: "".to_string(),
                });
            }

            finals.push(FinalMetadata {
                public_key: transaction.meta_contract_id,
                alias: "mentions".to_string(),
                content,
                version: parent_cid,
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
    pub fn curl(cmd: Vec<String>) -> MountedBinaryResult;
}

/**
 * For now leaving it empty. Freedom of speech
 */
pub fn is_profane(text: &str) -> bool {
    let profane_words = vec!["", ""];
    profane_words.iter().any(|&word| {
        if word != "" {
            return text.contains(word);
        }
        false
    })
}

pub fn is_nft_storage_link(link: &str) -> bool {
    link == "" || link.starts_with("https://nftstorage.link/ipfs/")
}

/**
 * Get data from json rpc
 */
pub fn fetch(data: String, url: String) -> String {
    let cmd = vec![
        String::from("curl"),
        String::from("-X"),
        String::from("POST"),
        String::from("-H"),
        String::from("Content-type: application/json"),
        String::from("-d"),
        data,
        url,
    ];

    let result = curl(cmd);
    String::from_utf8(result.stdout).unwrap()
}

pub fn make_search_metadatas_body(args: HashMap<String, String>) -> String {
    let mut query: Vec<FilterQuery> = Vec::new();

    for (key, value) in args.iter() {
        query.push(FilterQuery {
            column: key.to_string(),
            op: String::from("="),
            query: value.to_string(),
        })
    }

    let params = JSONRPCFilter {
        query,
        ordering: Vec::new(),
        from: 0,
        to: 0,
    };

    let body = JSONRPCBody {
        jsonrpc: String::from("2.0"),
        method: String::from("search_metadatas"),
        params,
        id: String::from("1"),
    };

    serde_json::to_string(&body).unwrap_or_default()
}

use crate::actors::chain_manager::messages::AddNewBlock;
use actix::System;
#[cfg(test)]
type ChainManager = actix::actors::mocker::Mocker<crate::actors::chain_manager::ChainManager>;
#[cfg(not(test))]
use crate::actors::chain_manager::ChainManager;
use jsonrpc_core::{IoHandler, Params, Value};
use log::info;
use serde_derive::{Deserialize, Serialize};
use witnet_data_structures::chain::Block;

/// Define the JSON-RPC interface:
/// All the methods available through JSON-RPC
pub fn jsonrpc_io_handler() -> IoHandler<()> {
    let mut io = IoHandler::new();

    io.add_method("inventory", |params: Params| inventory(params.parse()?));

    io
}

/// Inventory element: block, tx, etc
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum InventoryItem {
    /// Error
    #[serde(rename = "error")]
    Error,
    /// Transaction
    #[serde(rename = "tx")]
    Tx,
    /// Block
    #[serde(rename = "block")]
    Block(Block),
    /// Data request
    #[serde(rename = "data_request")]
    DataRequest,
    /// Data result
    #[serde(rename = "data_result")]
    DataResult,
}

/// Make the node process, validate and potentially broadcast a new inventory entry.
///
/// Input: the JSON serialization of a well-formed inventory entry
///
/// Returns a boolean indicating success.
/* Test string:
{"jsonrpc": "2.0","method": "inventory","params": {"block": {"block_header":{"version":1,"beacon":{"checkpoint":2,"hash_prev_block": {"SHA256": [4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4]}},"hash_merkle_root":{"SHA256":[3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3]}},"proof":{"block_sig": null,"influence":99999}"txns":[null]}},"id": 1}
*/
pub fn inventory(inv_elem: InventoryItem) -> Result<Value, jsonrpc_core::Error> {
    match inv_elem {
        InventoryItem::Block(block) => {
            info!("Got block from JSON-RPC. Sending AnnounceItems message.");

            // Get SessionsManager's address
            let chain_manager_addr = System::current().registry().get::<ChainManager>();
            // If this function was called asynchronously, it could wait for the result
            // But it's not so we just assume success
            chain_manager_addr.do_send(AddNewBlock { block });

            // Returns a boolean indicating success
            Ok(Value::Bool(true))
        }
        inv_elem => {
            info!(
                "Invalid type of inventory item from JSON-RPC: {:?}",
                inv_elem
            );
            Err(jsonrpc_core::Error::invalid_params(
                "Item type not implemented",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_parse_error() {
        // An empty message should return a parse error
        let empty_string = "";
        let parse_error =
            r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#
                .to_string();
        let io = jsonrpc_io_handler();
        let response = io.handle_request_sync(empty_string);
        assert_eq!(response, Some(parse_error));
    }

    #[test]
    fn inventory_method() {
        // The expected behaviour of the inventory method
        use witnet_data_structures::chain::*;

        let block = Block {
            block_header: BlockHeader {
                version: 1,
                beacon: CheckpointBeacon {
                    checkpoint: 2,
                    hash_prev_block: Hash::SHA256([4; 32]),
                },
                hash_merkle_root: Hash::SHA256([3; 32]),
            },
            proof: LeadershipProof {
                block_sig: None,
                influence: 99999,
            },
            txns: vec![Transaction],
        };

        let inv_elem = InventoryItem::Block(block.clone());
        let s = serde_json::to_string(&inv_elem).unwrap();
        let msg = format!(
            r#"{{"jsonrpc":"2.0","method":"inventory","params":{},"id":1}}"#,
            s
        );

        // Start an empty actix system
        // This one line is all that is needed to test a function which uses
        // do_send and never waits for the response. The handlers will never be
        // executed, so even the Mocker is not needed
        let system = System::new("test");

        use actix::Actor;
        use std::cell::Cell;
        use std::rc::Rc;
        // Call count. This could be implemented inside Mocker to avoid duplication.
        // Or maybe just do a SystemMocker to avoid having to define mockers for each actor
        let call_count = Rc::new(Cell::new(0));
        let call_count2 = Rc::clone(&call_count);
        // Setup the mocker actor
        // https://github.com/actix/actix/commit/dfdafda274a4761aea40051141a820170e96de80
        let mocker_addr = ChainManager::mock(Box::new(move |msg, _ctx| {
            // One if let branch for each type of message
            if let Some(b) = msg.downcast_ref::<AddNewBlock>() {
                call_count2.set(call_count2.get() + 1);
                // This is the only piece of code "unique" to this test, the rest
                // would be copied exactly to other tests
                {
                    // Check that the block is sent correctly
                    assert_eq!(b.block, block);
                }
                // Stop the system after receiving the first message
                if call_count2.get() >= 1 {
                    System::current().stop();
                }
                // Even if the system will stop, this return type must be valid
                // Box::new(None) does not work, but we only need to set the type
                // signature, the actual value will not be checked, so we just set it to zero.
                // For more information about runtime typing,
                // see `downcast_ref` in `std::any::Any`
                let res: <AddNewBlock as actix::Message>::Result = unsafe { std::mem::zeroed() };
                Box::new(Some(res))
            } else {
                panic!("Invalid message received");
            }
        }))
        .start();
        // Important: add the modified mocker into the registry
        System::current().registry().set(mocker_addr);

        // Expected result: true
        let expected = r#"{"jsonrpc":"2.0","result":true,"id":1}"#.to_string();
        let io = jsonrpc_io_handler();
        let response = io.handle_request_sync(&msg);
        assert_eq!(response, Some(expected));

        system.run();
        assert_eq!(call_count.get(), 1);
    }

    #[test]
    fn inventory_invalid_params() {
        // What happens when the inventory method is called with an invalid parameter?
        let msg = r#"{"jsonrpc":"2.0","method":"inventory","params":{ "header": 0 },"id":1}"#;
        let expected = r#"{"jsonrpc":"2.0","error":{"code":-32602,"message":"Invalid params: unknown variant `header`, expected one of"#.to_string();
        let io = jsonrpc_io_handler();
        let response = io.handle_request_sync(&msg);
        // Compare only the first N characters
        let response =
            response.map(|s| s.chars().take(expected.chars().count()).collect::<String>());
        assert_eq!(response, Some(expected));
    }

    #[test]
    fn inventory_unimplemented_type() {
        // What happens when the inventory method is called with an unimplemented type?
        let msg = r#"{"jsonrpc":"2.0","method":"inventory","params":{ "tx": null },"id":1}"#;
        let expected =
            r#"{"jsonrpc":"2.0","error":{"code":-32602,"message":"Item type not implemented"#
                .to_string();
        let io = jsonrpc_io_handler();
        let response = io.handle_request_sync(&msg);
        // Compare only the first N characters
        let response =
            response.map(|s| s.chars().take(expected.chars().count()).collect::<String>());
        assert_eq!(response, Some(expected));
    }

    #[test]
    fn serialize_block() {
        // Check that the serialization of `Block` doesn't change
        use witnet_data_structures::chain::*;

        let block = Block {
            block_header: BlockHeader {
                version: 1,
                beacon: CheckpointBeacon {
                    checkpoint: 2,
                    hash_prev_block: Hash::SHA256([4; 32]),
                },
                hash_merkle_root: Hash::SHA256([3; 32]),
            },
            proof: LeadershipProof {
                block_sig: None,
                influence: 99999,
            },
            txns: vec![Transaction],
        };
        let inv_elem = InventoryItem::Block(block);
        let s = serde_json::to_string(&inv_elem);
        let expected = r#"{"block":{"block_header":{"version":1,"beacon":{"checkpoint":2,"hash_prev_block":{"SHA256":[4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4]}},"hash_merkle_root":{"SHA256":[3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3]}},"proof":{"block_sig":null,"influence":99999},"txns":[null]}}"#;

        assert_eq!(s.unwrap(), expected);
    }
}

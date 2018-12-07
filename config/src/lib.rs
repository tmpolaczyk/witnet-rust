//! # Witnet-rust configuration library.
//!
//! This is the library code for reading and validating the
//! configuration read from an external data source. External data
//! sources and their format are handled through different loaders,
//! see the `witnet_config::loaders` module for more information.
//!
//! No matter which data source you use, ultimately all of them will
//! load the configuration as an instance of the `Config` struct which
//! is composed of other, more specialized, structs such as
//! `StorageConfig` and `ConnectionsConfig`. This instance is the one
//! you use in your Rust code to interact with the loaded
//! configuration.
#![cfg_attr(test, allow(dead_code, unused_macros, unused_imports))]
#![deny(rust_2018_idioms)]
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![deny(missing_docs)]

use failure;

pub mod config;
pub mod defaults;
pub mod loaders;

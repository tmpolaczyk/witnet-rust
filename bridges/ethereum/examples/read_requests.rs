use ethabi::Bytes;
use log::*;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::Path, sync::Arc};
use web3::{
    contract,
    contract::Contract,
    futures::{future, Future},
    types::H160,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    witnet_jsonrpc_addr: SocketAddr,
    eth_client_url: String,
    wbi_contract_addr: H160,
    eth_account: H160,
}

/// Load configuration from a file written in Toml format.
fn from_file<S: AsRef<Path>>(file: S) -> Result<Config, toml::de::Error> {
    use std::fs::File;
    use std::io::Read;

    let f = file.as_ref();
    let mut contents = String::new();

    debug!("Loading config from `{}`", f.to_string_lossy());

    let mut file = File::open(file).unwrap();
    file.read_to_string(&mut contents).unwrap();
    toml::from_str(&contents)
}

fn read_config() -> Config {
    from_file("witnet_ethereum_bridge.toml").unwrap()
}

fn eth_event_stream(
    config: Arc<Config>,
    web3: web3::Web3<web3::transports::Http>,
) -> impl Future<Item = (), Error = ()> {
    // Example from
    // https://github.com/tomusdrw/rust-web3/blob/master/examples/simple_log_filter.rs

    let accounts = web3.eth().accounts().wait().unwrap();
    debug!("Web3 accounts: {:?}", accounts);

    // Why read files at runtime when you can read files at compile time
    let contract_abi_json: &[u8] = include_bytes!("../wbi_abi.json");
    let contract_abi = ethabi::Contract::load(contract_abi_json).unwrap();
    let contract_address = config.wbi_contract_addr;
    let contract = Contract::new(web3.eth(), contract_address, contract_abi.clone());

    // TODO: how to access the public mapping "requests"?
    // (this doesn't work)
    let requests: Bytes = contract
        .query(
            "requests",
            (),
            accounts[0],
            contract::Options::with(|opt| {
                opt.value = Some(1000.into());
                opt.gas = Some(1_000_000.into());
            }),
            None,
        )
        .wait()
        .unwrap();
    info!("Got requests: {:?}", requests);

    future::finished(())
}

fn init_logger() {
    // Debug log level by default
    let mut log_level = log::LevelFilter::Debug;
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        if rust_log.contains("witnet") {
            log_level = env_logger::Logger::from_default_env().filter();
        }
    }

    env_logger::Builder::from_env(env_logger::Env::default())
        .filter_module("post_dr", log_level)
        .init();
}

fn main() {
    init_logger();
    let config = Arc::new(read_config());
    let (_eloop, web3_http) = web3::transports::Http::new(&config.eth_client_url).unwrap();
    let web3 = web3::Web3::new(web3_http);

    let ees = eth_event_stream(Arc::clone(&config), web3);

    tokio::run(future::ok(()).map(move |_| {
        tokio::spawn(ees);
    }));
}
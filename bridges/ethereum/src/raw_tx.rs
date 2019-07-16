use ethereum_tx_sign::RawTransaction;
use futures::Future;
use std::time::Duration;
use web3::api::Namespace;
use web3::contract::tokens::Tokenize;
use web3::contract::{Contract, Options};
use web3::types::{Address, BlockNumber, Bytes, CallRequest, TransactionReceipt, H256};
use web3::{confirm, Transport};

/// Extensions to Contract struct in web3 crate
pub trait BuildRawTransaction<T: Transport + Send + 'static> {
    /// Same as `call`, but sign the transaction locally with the given private key
    fn call_raw<P>(
        &self,
        func: &str,
        params: P,
        from: Address,
        private_key: H256,
        options: Options,
    ) -> Box<dyn Future<Item = H256, Error = web3::Error> + Send>
    where
        P: Tokenize,
        T::Out: Send;

    /// Same as `call_with_confirmations`, but sign the transaction locally with the given private key
    fn call_with_confirmations_raw<P>(
        &self,
        func: &str,
        params: P,
        from: Address,
        private_key: H256,
        options: Options,
        confirmations: usize,
    ) -> Box<dyn Future<Item = TransactionReceipt, Error = web3::Error> + Send>
    where
        P: Tokenize,
        T::Out: Send;
}

impl<T: Transport + Send + 'static> BuildRawTransaction<T> for Contract<T> {
    fn call_raw<P>(
        &self,
        func: &str,
        params: P,
        from: Address,
        private_key: H256,
        options: Options,
    ) -> Box<dyn Future<Item = H256, Error = web3::Error> + Send>
    where
        P: Tokenize,
        T::Out: Send,
    {
        self.abi()
            .function(func)
            .and_then(|function| function.encode_input(&params.into_tokens()))
            .map(move |data| {
                // To build a raw transaction, we need:
                // nonce: eth.transaction_count()
                // gas_price: eth.gas_price()
                // gas: eth.estimate_gas()

                let eth = self.eth().clone();
                let eth1 = eth.clone();
                let eth2 = eth.clone();
                let to = self.address();
                let value = options.value;
                let fut_nonce = eth.transaction_count(from, Some(BlockNumber::Pending));
                let fut_gas_price = eth.gas_price();
                let fut: Box<dyn Future<Item = H256, Error = web3::Error> + Send> = Box::new(
                    fut_nonce
                        .join(fut_gas_price)
                        .and_then(move |(nonce, gas_price)| {
                            let call_request = CallRequest {
                                from: Some(from),
                                to,
                                gas: None,
                                gas_price: Some(gas_price),
                                value,
                                data: Some(Bytes(data.clone())),
                            };
                            eth1.estimate_gas(call_request, None)
                                .map(move |gas| (nonce, gas_price, gas, data))
                        })
                        .and_then(move |(nonce, gas_price, gas, data)| {
                            let raw_tx = RawTransaction {
                                nonce,
                                to: Some(to),
                                value: value.unwrap_or_default(),
                                gas_price,
                                gas,
                                data,
                            };
                            let chain_id = 0x01;
                            let signed_tx = raw_tx.sign(&private_key, chain_id);
                            /*
                            self.eth
                                .send_transaction(TransactionRequest {
                                    from,
                                    to: Some(self.address().clone()),
                                    gas: options.gas,
                                    gas_price: options.gas_price,
                                    value: options.value,
                                    nonce: options.nonce,
                                    data: Some(Bytes(data)),
                                    condition: options.condition,
                                })
                                .into()
                                */
                            eth2.send_raw_transaction(signed_tx.into())
                        }),
                );

                fut
            })
            // TODO: error handling
            .unwrap_or_else(|_e| Box::new(futures::failed(web3::Error::Internal)))
    }

    /// Same as `call_with_confirmations`, but sign the transaction locally with the given private key
    fn call_with_confirmations_raw<P>(
        &self,
        func: &str,
        params: P,
        from: Address,
        private_key: H256,
        options: Options,
        confirmations: usize,
    ) -> Box<dyn Future<Item = TransactionReceipt, Error = web3::Error> + Send>
    where
        P: Tokenize,
        T::Out: Send,
    {
        let poll_interval = Duration::from_secs(1);

        self.abi()
            .function(func)
            .and_then(|function| function.encode_input(&params.into_tokens()))
            .map(move |data| {
                // To build a raw transaction, we need:
                // nonce: eth.transaction_count()
                // gas_price: eth.gas_price()
                // gas: eth.estimate_gas()

                let eth = self.eth().clone();
                let eth1 = eth.clone();
                let eth2 = eth.clone();
                let to = self.address();
                let value = options.value;
                let fut_nonce = eth.transaction_count(from, Some(BlockNumber::Pending));
                let fut_gas_price = eth.gas_price();
                let fut: Box<dyn Future<Item = TransactionReceipt, Error = web3::Error> + Send> =
                    Box::new(
                        fut_nonce
                            .join(fut_gas_price)
                            .and_then(move |(nonce, gas_price)| {
                                let call_request = CallRequest {
                                    from: Some(from),
                                    to,
                                    gas: None,
                                    gas_price: Some(gas_price),
                                    value,
                                    data: Some(Bytes(data.clone())),
                                };
                                eth1.estimate_gas(call_request, None)
                                    .map(move |gas| (nonce, gas_price, gas, data))
                            })
                            .and_then(move |(nonce, gas_price, gas, data)| {
                                let raw_tx = RawTransaction {
                                    nonce,
                                    to: Some(to),
                                    value: value.unwrap_or_default(),
                                    gas_price,
                                    gas,
                                    data,
                                };
                                let chain_id = 0x01;
                                let signed_tx = raw_tx.sign(&private_key, chain_id);
                                /*
                                self.eth
                                    .send_transaction(TransactionRequest {
                                        from,
                                        to: Some(self.address().clone()),
                                        gas: options.gas,
                                        gas_price: options.gas_price,
                                        value: options.value,
                                        nonce: options.nonce,
                                        data: Some(Bytes(data)),
                                        condition: options.condition,
                                    })
                                    .into()
                                    */
                                confirm::send_raw_transaction_with_confirmation(
                                    eth2.transport().clone(),
                                    signed_tx.into(),
                                    poll_interval,
                                    confirmations,
                                )
                            }),
                    );

                fut
            })
            // TODO: error handling
            .unwrap_or_else(|_e| Box::new(futures::failed(web3::Error::Internal)))
    }
}

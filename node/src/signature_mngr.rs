//! # Signature Manager
//!
//! This module provides a Signature Manager, which, after being
//! initialized with a key, can be used repeatedly to sign data with
//! that key.
use actix::prelude::*;
use failure;
use failure::bail;
use futures::future::Future;

use witnet_crypto::{
    key::{SignContext, PK, SK},
    signature,
};
use witnet_data_structures::chain::{Hash, Hashable, KeyedSignature, PublicKey, Signature};

/// Start the signature manager
pub fn start() {
    let addr = SignatureManager::start_default();
    actix::System::current().registry().set(addr);
}

/// Set the key used to sign
pub fn set_key(key: SK) -> impl Future<Item = (), Error = failure::Error> {
    let addr = actix::System::current()
        .registry()
        .get::<SignatureManager>();
    addr.send(SetKey(key)).flatten()
}

/// Sign a piece of data with the stored key.
///
/// This might fail if the manager has not been initialized with a key
pub fn sign<T>(data: &T) -> impl Future<Item = KeyedSignature, Error = failure::Error>
where
    T: Hashable,
{
    let addr = actix::System::current()
        .registry()
        .get::<SignatureManager>();
    let Hash::SHA256(data_hash) = data.hash();

    addr.send(Sign(data_hash.to_vec())).flatten()
}

#[derive(Debug, Default)]
struct SignatureManager {
    keypair: Option<(SK, PK)>,
}

struct SetKey(SK);
struct Sign(Vec<u8>);

impl Actor for SignatureManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::debug!("Signature Manager actor has been started!");
    }
}

impl Supervised for SignatureManager {}

impl SystemService for SignatureManager {}

impl Message for SetKey {
    type Result = Result<(), failure::Error>;
}

impl Message for Sign {
    type Result = Result<KeyedSignature, failure::Error>;
}

impl Handler<SetKey> for SignatureManager {
    type Result = <SetKey as Message>::Result;

    fn handle(&mut self, SetKey(secret_key): SetKey, _ctx: &mut Self::Context) -> Self::Result {
        let public_key = PK::from_secret_key(&SignContext::signing_only(), &secret_key);
        self.keypair = Some((secret_key, public_key));

        log::info!("Signature Manager received a key and is ready to sign");

        Ok(())
    }
}

impl Handler<Sign> for SignatureManager {
    type Result = <Sign as Message>::Result;

    fn handle(&mut self, Sign(data): Sign, _ctx: &mut Self::Context) -> Self::Result {
        match self.keypair {
            Some((secret, public)) => {
                let signature = signature::sign(secret, &data);
                let keyed_signature = KeyedSignature {
                    signature: Signature::from(signature),
                    public_key: PublicKey::from(public),
                };

                Ok(keyed_signature)
            }
            None => bail!("Signature Manager cannot sign because it contains no key"),
        }
    }
}

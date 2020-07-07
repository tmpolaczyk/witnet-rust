use actix::prelude::*;

use super::PeersManager;
use crate::{actors::storage_keys, config_mngr, storage_mngr};
use witnet_p2p::peers::Peers;

/// Make actor from PeersManager
impl Actor for PeersManager {
    /// Every actor has to provide execution Context in which it can run.
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::debug!("Peers Manager actor has been started!");

        // Send message to config manager and process response
        config_mngr::get()
            .map_err(|e| log::error!("Failed to read config: {}", e))
            .into_actor(self)
            .and_then(|config, act, _ctx| {
                let magic = config.consensus_constants.get_magic();
                act.set_magic(magic);

                storage_mngr::get::<_, Peers>(&storage_keys::peers_key(magic))
                    .map_err(|e| log::error!("Couldn't get peers from storage: {}", e))
                    .map(|peers_from_storage| (config, peers_from_storage))
                    .into_actor(act)
            })
            .and_then(|(config, peers_from_storage), act, ctx| {
                // peers_from_storage can be None if the storage does not contain that key
                if let Some(peers_from_storage) = peers_from_storage {
                    // Add all the peers from storage
                    act.peers = peers_from_storage;
                }

                // Set server address
                act.peers.set_server(config.connections.server_addr);

                // Set bucketing "ice" and update period
                act.peers
                    .set_ice_period(config.connections.bucketing_ice_period);
                act.bucketing_update_period = config.connections.bucketing_update_period;

                // Add peers from config
                let peers_from_config: Vec<_> =
                    config.connections.known_peers.iter().cloned().collect();
                log::info!(
                    "Adding the following peer addresses from config: {:?}",
                    peers_from_config
                );
                match act.peers.add_to_new(peers_from_config, None) {
                    Ok(_duplicated_peers) => {}
                    Err(e) => log::error!("Error when adding peer addresses from config: {}", e),
                }

                // Start the storage peers process on PeersManager start
                act.persist_peers(ctx, config.connections.storage_peers_period);

                // Start the feeleer peers process on PeersManager start
                act.feeler(ctx, config.connections.feeler_peers_period);

                fut::ok(())
            })
            .wait(ctx);
    }
}

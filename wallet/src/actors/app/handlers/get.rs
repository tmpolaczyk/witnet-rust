use actix::prelude::*;
use serde::{Deserialize, Serialize};
use witnet_futures_utils::ActorFutureExt;

use crate::{actors::app, types};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRequest {
    session_id: types::SessionId,
    wallet_id: String,
    key: String,
}

#[derive(Serialize)]
pub struct GetResponse {
    value: Option<types::RpcValue>,
}

impl Message for GetRequest {
    type Result = app::Result<GetResponse>;
}

impl Handler<GetRequest> for app::App {
    type Result = app::ResponseActFuture<GetResponse>;

    fn handle(&mut self, req: GetRequest, _ctx: &mut Self::Context) -> Self::Result {
        let f = self
            .get(req.session_id, req.wallet_id, req.key)
            .map_ok(|value, _, _| GetResponse { value });

        Box::pin(f)
    }
}

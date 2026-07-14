use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;

/// `client.accounts()`: connected social accounts.
#[derive(Debug, Clone, Copy)]
pub struct Accounts<'a> {
    pub(crate) client: &'a Client,
}

impl Accounts<'_> {
    /// `GET /accounts` - the workspace's connected social accounts. Each
    /// account carries a `status` that flips to `"needs_reconnect"` when the
    /// platform has revoked or expired its OAuth token.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/accounts", Vec::new()).await
    }

    /// `GET /accounts/:id` - a single connected account.
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/accounts/{}", encode_path_segment(id)), Vec::new())
            .await
    }
}

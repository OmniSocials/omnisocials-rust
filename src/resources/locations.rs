use serde_json::Value;

use crate::client::Client;
use crate::error::Error;

/// `client.locations()`: Instagram location tagging (Facebook Places).
#[derive(Debug, Clone, Copy)]
pub struct Locations<'a> {
    pub(crate) client: &'a Client,
}

impl Locations<'_> {
    /// `GET /locations/search?q=` - search Facebook Places for Instagram
    /// location tagging. Use a result's `id` as `location_id` on a post.
    pub async fn search(&self, query: &str) -> Result<Value, Error> {
        self.client
            .get("/locations/search", vec![("q", query.to_owned())])
            .await
    }

    /// `GET /locations/validate?id=` - check whether a Facebook Place id is
    /// a valid Instagram location before using it as `location_id`.
    pub async fn validate(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get("/locations/validate", vec![("id", id.to_owned())])
            .await
    }
}

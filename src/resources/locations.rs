use serde_json::Value;

use crate::client::Client;
use crate::error::Error;
use crate::types::SearchLocationsParams;

/// `client.locations()`: location search for post tagging - Instagram
/// (Facebook Places) and Threads (Meta's Threads location catalog). The two
/// sources use different ids: a Facebook Place id is not a Threads location
/// id.
#[derive(Debug, Clone, Copy)]
pub struct Locations<'a> {
    pub(crate) client: &'a Client,
}

impl Locations<'_> {
    /// `GET /locations/search?q=` - search Facebook Places for Instagram
    /// location tagging (the default source). Use a result's `id` as
    /// `location_id` on a post. For Threads locations, use
    /// [`search_with`](Locations::search_with).
    pub async fn search(&self, query: &str) -> Result<Value, Error> {
        self.client
            .get("/locations/search", vec![("q", query.to_owned())])
            .await
    }

    /// `GET /locations/search` with the full parameter set. Set `platform`
    /// to `"threads"` to search Meta's Threads location catalog instead of
    /// Facebook Places; pass either `q` or the `latitude` + `longitude` pair
    /// (coordinates are Threads only).
    ///
    /// The Threads response shape differs from Instagram's:
    /// `{"locations": [{"id", "name", "address", "city", "country",
    /// "latitude", "longitude"}]}` (all fields but `id` nullable) or
    /// `{"error": {"code", "message"}}` where `code` is one of
    /// `not_available`, `threads_not_connected`, `threads_reauth_required`
    /// (the connection lacks the `threads_location_tagging` permission;
    /// reconnect Threads), or `platform_error`. Pass a Threads result's `id`
    /// as `threads.location_id` on post create/update. Threads location
    /// tagging is currently rolling out; until Meta approves the permissions
    /// it is disabled on production and calls return a clear error.
    pub async fn search_with(&self, params: SearchLocationsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(q) = params.q {
            query.push(("q", q));
        }
        if let Some(platform) = params.platform {
            query.push(("platform", platform));
        }
        if let Some(latitude) = params.latitude {
            query.push(("latitude", latitude.to_string()));
        }
        if let Some(longitude) = params.longitude {
            query.push(("longitude", longitude.to_string()));
        }
        self.client.get("/locations/search", query).await
    }

    /// `GET /locations/validate?id=` - check whether a Facebook Place id is
    /// a valid Instagram location before using it as `location_id`.
    pub async fn validate(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get("/locations/validate", vec![("id", id.to_owned())])
            .await
    }
}

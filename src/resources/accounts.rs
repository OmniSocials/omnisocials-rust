use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::x_statistics_params::UpdateAccountParams;

/// `client.accounts()`: connected social accounts.
#[derive(Debug, Clone, Copy)]
pub struct Accounts<'a> {
    pub(crate) client: &'a Client,
}

impl Accounts<'_> {
    /// `GET /accounts` - the workspace's connected social accounts. Each
    /// account carries a `status`: `"active"`, `"needs_reconnect"` when the
    /// platform has revoked or expired its OAuth token (Bluesky: the sign-in
    /// could not be renewed), or `"pick_pending"` (Facebook, OmniSocials
    /// Connect customers only: the Facebook login manages several Pages and
    /// none is chosen yet; posts fail until the Page is chosen by connecting
    /// Facebook again and picking the Page). The booleans `needs_reconnect` and `pick_pending`
    /// mirror it. When `pick_pending` is `true`, `username`, `display_name`
    /// and `profile_picture` name the person who signed in, not a Page.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/accounts", Vec::new()).await
    }

    /// `GET /accounts/:id` - a single connected account. An X account also
    /// carries `x_statistics`: `posts`, `followers`, `post_schedule`,
    /// `source`, `billing`, `state` and `paused_reason` (null when it could
    /// not be read).
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/accounts/{}", encode_path_segment(id)), Vec::new())
            .await
    }

    /// `PATCH /accounts/:id` - change how often the X statistics of an X
    /// account are read. Only the fields you set on
    /// [`crate::XStatisticsInput`] are sent; `x_statistics: None` is sent as
    /// `null` (OmniSocials Connect keys: follow the customer's setting
    /// again). X bills every read and OmniSocials passes X's price on as
    /// credits (half a credit per post check, plus half for a video or GIF,
    /// 1 credit per follower count). Needs the opt-in scope `accounts:write`
    /// and billing rights (manage_billing) for the key's creator. Returns
    /// `{"data": account}` with the new `x_statistics`. Errors: 400
    /// `validation_error` / `schedule_invalid` / `not_supported` (not an X
    /// account), 403 `forbidden`, 404 `not_found`.
    pub async fn update(&self, id: &str, params: UpdateAccountParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/accounts/{}", encode_path_segment(id)), &params)
            .await
    }
}

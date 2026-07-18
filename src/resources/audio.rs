use serde_json::Value;

use crate::client::Client;
use crate::error::Error;

/// `client.audio()`: Instagram Reels licensed audio search (Meta catalog).
#[derive(Debug, Clone, Copy)]
pub struct Audio<'a> {
    pub(crate) client: &'a Client,
}

impl Audio<'_> {
    /// `GET /audio/search?q=&type=` - search Meta's licensed audio catalog
    /// for Instagram Reels. Pass `None` as `query` for trending audio;
    /// `audio_type` is `"music"` (server default) or `"original_sound"`.
    /// Use a result's `audio_id` as `instagram.audio_id` on a reel post
    /// (with optional `instagram.audio_volume` / `instagram.video_volume`,
    /// integers 0-100). `preview_url` in results is temporary (~1.5 days);
    /// never persist it. Note: unlike the usual envelope, `error` here is a
    /// plain string set when search is unavailable (e.g. no Facebook account
    /// connected), with `data` then empty and `needsFacebook` set.
    pub async fn search(
        &self,
        query: Option<&str>,
        audio_type: Option<&str>,
    ) -> Result<Value, Error> {
        let mut params = Vec::new();
        if let Some(query) = query {
            params.push(("q", query.to_owned()));
        }
        if let Some(audio_type) = audio_type {
            params.push(("type", audio_type.to_owned()));
        }
        self.client.get("/audio/search", params).await
    }
}

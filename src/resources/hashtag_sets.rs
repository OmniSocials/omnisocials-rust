use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{CreateHashtagSetParams, UpdateHashtagSetParams};

/// `client.hashtag_sets()`: saved, reusable hashtag groups. Apply a set to a
/// post at create time via [`crate::CreatePostParams::hashtag_set`] (name,
/// case-insensitive) or [`crate::CreatePostParams::hashtag_set_id`].
#[derive(Debug, Clone, Copy)]
pub struct HashtagSets<'a> {
    pub(crate) client: &'a Client,
}

impl HashtagSets<'_> {
    /// `GET /hashtag-sets` - list the workspace's saved hashtag sets.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/hashtag-sets", Vec::new()).await
    }

    /// `GET /hashtag-sets/:id` - fetch a single hashtag set.
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/hashtag-sets/{}", encode_path_segment(id)), Vec::new())
            .await
    }

    /// `POST /hashtag-sets` - create a hashtag set. `hashtags` is a list of
    /// tags, or one string of tags.
    pub async fn create(&self, params: CreateHashtagSetParams) -> Result<Value, Error> {
        self.client.post_json("/hashtag-sets", &params).await
    }

    /// `PATCH /hashtag-sets/:id` - rename (`name`) and/or replace the tags
    /// (`hashtags` replaces the FULL list).
    pub async fn update(&self, id: &str, params: UpdateHashtagSetParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/hashtag-sets/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /hashtag-sets/:id` - delete a hashtag set. Resolves to
    /// [`Value::Null`] (204).
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/hashtag-sets/{}", encode_path_segment(id)))
            .await
    }
}

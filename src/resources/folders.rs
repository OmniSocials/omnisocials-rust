use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{CreateFolderParams, UpdateFolderParams};

/// `client.folders()`: Media Library folders.
#[derive(Debug, Clone, Copy)]
pub struct Folders<'a> {
    pub(crate) client: &'a Client,
}

impl Folders<'_> {
    /// `GET /folders` - all folders (flat list; build the tree via `parent_id`).
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/folders", Vec::new()).await
    }

    /// `POST /folders` - create a folder (optionally nested under `parent_id`).
    pub async fn create(&self, params: CreateFolderParams) -> Result<Value, Error> {
        self.client.post_json("/folders", &params).await
    }

    /// `PATCH /folders/:id` - rename (`name`) and/or move (`parent_id`, or
    /// JSON `null` for the top level). Cycles are rejected.
    pub async fn update(&self, id: &str, params: UpdateFolderParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/folders/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /folders/:id` - delete a folder. Media is preserved: files
    /// move to the root and subfolders move up to the deleted folder's
    /// parent. Resolves to [`Value::Null`] (204).
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/folders/{}", encode_path_segment(id)))
            .await
    }
}

use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{
    CheckMediaParams, ListMediaParams, UpdateMediaParams, UploadMediaFromBase64Params,
    UploadMediaFromUrlParams, UploadMediaParams,
};

/// `client.media()`: the workspace Media Library.
#[derive(Debug, Clone, Copy)]
pub struct Media<'a> {
    pub(crate) client: &'a Client,
}

impl Media<'_> {
    /// `GET /media` - list the media library (newest first).
    pub async fn list(&self, params: ListMediaParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = params.offset {
            query.push(("offset", offset.to_string()));
        }
        if let Some(search) = params.search {
            query.push(("search", search));
        }
        if let Some(folder_id) = params.folder_id {
            query.push(("folder_id", folder_id));
        }
        self.client.get("/media", query).await
    }

    /// `GET /media/:id` - fetch a single media item.
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/media/{}", encode_path_segment(id)), Vec::new())
            .await
    }

    /// `POST /media/upload` - upload raw bytes as multipart form data (max
    /// 50MB for images, 100MB request cap overall; use [`Self::upload_from_url`]
    /// or [`Self::create_upload_url`] for larger videos, up to 1GB).
    ///
    /// A PDF is rasterized into image slides and returned as a carousel
    /// (`slides` + `media_ids` alongside `data`).
    pub async fn upload(&self, params: UploadMediaParams) -> Result<Value, Error> {
        let mut fields = Vec::new();
        if let Some(name) = params.name {
            fields.push(("name", name));
        }
        if let Some(folder) = params.folder {
            fields.push(("folder", folder));
        }
        if let Some(folder_id) = params.folder_id {
            fields.push(("folder_id", folder_id));
        }
        let filename = if params.filename.is_empty() {
            "upload.bin".to_owned()
        } else {
            params.filename
        };
        self.client
            .post_multipart("/media/upload", params.file, filename, fields)
            .await
    }

    /// `POST /media/upload-from-url` - the server fetches a public URL
    /// (files up to 1GB; large videos finish processing in the background
    /// and come back with status `"processing"`).
    pub async fn upload_from_url(&self, params: UploadMediaFromUrlParams) -> Result<Value, Error> {
        self.client.post_json("/media/upload-from-url", &params).await
    }

    /// `POST /media/upload-from-base64` - upload base64-encoded file data.
    pub async fn upload_from_base64(
        &self,
        params: UploadMediaFromBase64Params,
    ) -> Result<Value, Error> {
        self.client.post_json("/media/upload-from-base64", &params).await
    }

    /// `POST /media/create-upload-url` - mint a one-time presigned upload URL
    /// for large files (up to 1GB). POST the file as multipart form data
    /// (field name `"file"`) to the returned `upload_url` within
    /// `expires_in_seconds`; no auth headers are needed on that second request.
    pub async fn create_upload_url(&self) -> Result<Value, Error> {
        self.client.post_empty("/media/create-upload-url").await
    }

    /// `POST /media/check` - preflight a file's compatibility with the
    /// workspace's connected platforms BEFORE uploading. Provide one of: a
    /// public `url`, an existing `media_id`, or `size_bytes` + `mime`.
    pub async fn check(&self, params: CheckMediaParams) -> Result<Value, Error> {
        self.client.post_json("/media/check", &params).await
    }

    /// `PATCH /media/:id` - rename a file and/or move it into a folder.
    pub async fn update(&self, id: &str, params: UpdateMediaParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/media/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /media/:id` - delete a media file. Resolves to
    /// [`Value::Null`] (204). Fails with 409 `media_in_use` when the file is
    /// attached to a scheduled or publishing post.
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/media/{}", encode_path_segment(id)))
            .await
    }
}

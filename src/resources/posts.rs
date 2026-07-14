use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{CreatePostParams, ListPostsParams, RecentPlatformPostsParams, UpdatePostParams};

/// `client.posts()`: create, schedule, publish, and manage posts.
#[derive(Debug, Clone, Copy)]
pub struct Posts<'a> {
    pub(crate) client: &'a Client,
}

impl Posts<'_> {
    /// `GET /posts` - list posts in the workspace (newest first).
    pub async fn list(&self, params: ListPostsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(status) = params.status {
            query.push(("status", status));
        }
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = params.offset {
            query.push(("offset", offset.to_string()));
        }
        self.client.get("/posts", query).await
    }

    /// `GET /posts/:id` - fetch a single post.
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/posts/{}", encode_path_segment(id)), Vec::new())
            .await
    }

    /// `GET /posts/recent-platform` - recent posts fetched live from the
    /// connected platform APIs (including content published outside
    /// OmniSocials). The fallback for brand-new workspaces where `list` is
    /// empty. Requires the `analytics:read` scope.
    pub async fn recent_platform(&self, params: RecentPlatformPostsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(platforms) = params.platforms {
            query.push(("platforms", platforms.join(",")));
        }
        self.client.get("/posts/recent-platform", query).await
    }

    /// `POST /posts/create` - create a draft or scheduled post.
    pub async fn create(&self, params: CreatePostParams) -> Result<Value, Error> {
        self.client.post_json("/posts/create", &params).await
    }

    /// `POST /posts/create-and-publish` - create a post and publish it
    /// immediately (any `scheduled_at` is ignored).
    pub async fn create_and_publish(&self, params: CreatePostParams) -> Result<Value, Error> {
        self.client.post_json("/posts/create-and-publish", &params).await
    }

    /// `PATCH /posts/:id` - update a draft or scheduled post.
    pub async fn update(&self, id: &str, params: UpdatePostParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/posts/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /posts/:id` - delete a post. Resolves to [`Value::Null`] (204).
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/posts/{}", encode_path_segment(id)))
            .await
    }

    /// `POST /posts/:id/publish` - publish a draft or scheduled post now.
    pub async fn publish(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("/posts/{}/publish", encode_path_segment(id)))
            .await
    }
}

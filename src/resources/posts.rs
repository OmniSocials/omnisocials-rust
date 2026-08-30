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
    ///
    /// When the post targets X and its text (or any thread part) contains a
    /// URL, the response includes a top-level `warnings` array (sibling of
    /// `data`) with a `x_url_post_credits` entry carrying `credits_required`
    /// and `credits_balance`: X's link-post fee is passed through as prepaid
    /// credits, debited at publish time (from 2026-08-14). Credits are
    /// managed in the dashboard, not the API.
    ///
    /// Separately, from 2026-08-14, scheduling or publishing that same post
    /// can also be refused up front: if reserving this post's cost would
    /// push the company's total reserved credits past its balance, the call
    /// fails with [`Error::Api`] (status 402) and code
    /// `x_credits_insufficient`, carrying `credits_required`,
    /// `credits_balance`, and `credits_reserved` in the error body's
    /// `error.details`. Drafts (no `scheduled_at`) are never gated, and
    /// neither is any post publishing before 2026-08-14. The same gate
    /// applies to [`Self::update`] and [`Self::publish`].
    pub async fn create(&self, params: CreatePostParams) -> Result<Value, Error> {
        self.client.post_json("/posts/create", &params).await
    }

    /// `POST /posts/create-and-publish` - create a post and publish it
    /// immediately (any `scheduled_at` is ignored). See [`Self::create`] for
    /// the `warnings` array on X link posts.
    pub async fn create_and_publish(&self, params: CreatePostParams) -> Result<Value, Error> {
        self.client.post_json("/posts/create-and-publish", &params).await
    }

    /// `PATCH /posts/:id` - update a draft or scheduled post. For X posts
    /// this can fail with `402 x_credits_insufficient` under the same
    /// credit-reservation gate as [`Self::create`].
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
    /// For X posts this can fail with `402 x_credits_insufficient` under the
    /// same credit-reservation gate as [`Self::create`].
    pub async fn publish(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("/posts/{}/publish", encode_path_segment(id)))
            .await
    }

    /// `POST /posts/:id/retry` - retry the failed platforms of a `failed` or
    /// `warning` (partially failed) post, on the same post. Only the
    /// platforms that failed are re-published; platforms that already
    /// succeeded are never posted again. Asynchronous: a 200 means the retry
    /// is queued - poll `get` for the outcome. Max 3 retries per platform.
    pub async fn retry(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("/posts/{}/retry", encode_path_segment(id)))
            .await
    }

    /// `POST /posts/:id/approve` - approve the current step of a post's
    /// approval workflow, on behalf of the user who owns this API key. That
    /// user must be a listed approver for the workflow's CURRENT step -
    /// steps approve in order, so an approver on a later step gets a 403
    /// `forbidden` error until earlier steps clear. Only works on a post
    /// with `approval_status: "pending"`. If this is the last step, the
    /// post finalizes immediately (`scheduled` or `posting`); otherwise it
    /// stays `in_approval` and the next step's approvers are notified.
    pub async fn approve(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("/posts/{}/approve", encode_path_segment(id)))
            .await
    }

    /// `POST /posts/:id/reject` - reject a post's approval workflow, on
    /// behalf of the user who owns this API key. Same approver requirement
    /// as [`Self::approve`]. Unlike approval, a rejection stops the WHOLE
    /// workflow immediately (not just the current step) - the post's status
    /// becomes `rejected`. `comment` is optional (pass `None`) and, when
    /// given, is shown to the requester and other approvers in the post's
    /// review thread.
    pub async fn reject(&self, id: &str, comment: Option<&str>) -> Result<Value, Error> {
        match comment {
            Some(c) => {
                self.client
                    .post_json(
                        &format!("/posts/{}/reject", encode_path_segment(id)),
                        &serde_json::json!({ "comment": c }),
                    )
                    .await
            }
            None => {
                self.client
                    .post_empty(&format!("/posts/{}/reject", encode_path_segment(id)))
                    .await
            }
        }
    }
}

use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{
    ApprovePostParams, CreatePostParams, ListPostsParams, RecentPlatformPostsParams,
    RejectPostParams, UpdatePostParams,
};

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
        if let Some(start_date) = params.start_date {
            query.push(("start_date", start_date));
        }
        if let Some(end_date) = params.end_date {
            query.push(("end_date", end_date));
        }
        if let Some(tag) = params.tag {
            query.push(("tag", tag));
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
    ///
    /// The same `warnings` array carries one `image_fit` entry per channel
    /// decision on a feed image whose ratio is outside what the channel
    /// shows uncropped (`action` is `cropped`, `kept` or `crop_failed`, with
    /// `platform`, `media_url`, `ratio`, `target` and `crop_url`);
    /// `media_fit` on the params steers it per channel.
    ///
    /// A media id that is still processing (a large video is checked and
    /// converted after upload) answers `409 media_processing` and nothing is
    /// created: wait until `media().get(id)` says `ready`, then send the
    /// same request again. A file whose processing failed answers
    /// `400 invalid_media`: upload it again.
    pub async fn create(&self, params: CreatePostParams) -> Result<Value, Error> {
        self.client.post_json("/posts/create", &params).await
    }

    /// `POST /posts/create-and-publish` - create a post and publish it
    /// immediately (any `scheduled_at` is ignored). See [`Self::create`] for
    /// the `warnings` array on X link posts and the `409 media_processing` /
    /// `400 invalid_media` answers.
    pub async fn create_and_publish(&self, params: CreatePostParams) -> Result<Value, Error> {
        self.client.post_json("/posts/create-and-publish", &params).await
    }

    /// `PATCH /posts/:id` - update a draft or scheduled post. For X posts
    /// this can fail with `402 x_credits_insufficient` under the same
    /// credit-reservation gate as [`Self::create`]. Like [`Self::create`],
    /// the response can carry a top-level `warnings` array: `image_fit`
    /// entries when the update changes the images or `media_fit`
    /// (`media_fit` alone re-decides the stored images; `keep` gives the
    /// original back).
    ///
    /// 409 answers, where nothing is changed: `post_publishing` while the
    /// post is publishing (status `posting`) or a retry publishes one of its
    /// channels (poll [`Self::get`] until `published`, `warning` or
    /// `failed`; an update with only `tags` still works); `review_required`
    /// when changes were requested on the post (`approval_status`
    /// `rejected`) and the update would schedule it (it needs a new review:
    /// create it again with `approval_workflow_id`; saving it as a draft
    /// works); `media_processing` as on [`Self::create`].
    pub async fn update(&self, id: &str, params: UpdatePostParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/posts/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /posts/:id` - remove a post from OmniSocials (the live post
    /// stays on the platform). Resolves to [`Value::Null`] (204). While the
    /// post is publishing (status `posting`) or a retry publishes one of its
    /// channels, the answer is `409 post_publishing` and nothing is deleted:
    /// poll [`Self::get`] until `published`, `warning` or `failed`, then
    /// delete it.
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/posts/{}", encode_path_segment(id)))
            .await
    }

    /// `POST /posts/:id/publish` - publish a draft or scheduled post now.
    /// For X posts this can fail with `402 x_credits_insufficient` under the
    /// same credit-reservation gate as [`Self::create`]. A post on which
    /// changes were requested in its approval workflow (`approval_status`
    /// `rejected`) answers `409 review_required`: it publishes only after a
    /// new review (create it again with `approval_workflow_id`).
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
    /// While another retry runs (a second request, the dashboard, the
    /// automatic retry) the answer is `409 retry_in_progress` and nothing is
    /// queued: poll `get` and retry once the post is `failed` or `warning`
    /// again.
    pub async fn retry(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("/posts/{}/retry", encode_path_segment(id)))
            .await
    }

    /// `POST /posts/:id/approve` - approve the current step of a post's
    /// approval workflow, on behalf of the user who owns this API key. That
    /// user must be a listed approver for the workflow's CURRENT step:
    /// steps approve in order, so an approver on a later step gets a 403
    /// `forbidden` error until earlier steps clear. Only works on a post
    /// with `approval_status: "pending"`. If this is the last step, the
    /// post finalizes immediately (`scheduled` or `posting`); otherwise it
    /// stays `in_approval` and the next step's approvers are notified.
    ///
    /// This acts on whatever step is current; prefer [`Self::approve_with`]
    /// with the step you reviewed.
    pub async fn approve(&self, id: &str) -> Result<Value, Error> {
        self.approve_with(id, ApprovePostParams::default()).await
    }

    /// `POST /posts/:id/approve` with `step_order`: the `current_step` of
    /// [`Self::get_approval`], the step you reviewed. When the review is on
    /// another step now (somebody decided since you read it, or a retried
    /// call already went through), nothing is approved and the call returns
    /// [`Error::Api`] with status 409 and code `step_changed`;
    /// `body["error"]["current_step_order"]` is the step the review is on
    /// now. Read the approval again before you decide. With `step_order:
    /// None` the call approves whatever step is current, so a retry after a
    /// timeout can approve the next step too.
    pub async fn approve_with(&self, id: &str, params: ApprovePostParams) -> Result<Value, Error> {
        let path = format!("/posts/{}/approve", encode_path_segment(id));
        if params.step_order.is_none() {
            return self.client.post_empty(&path).await;
        }
        self.client.post_json(&path, &params).await
    }

    /// `POST /posts/:id/reject` - reject a post's approval workflow, on
    /// behalf of the user who owns this API key. Same approver requirement
    /// as [`Self::approve`]. Unlike approval, a rejection stops the WHOLE
    /// workflow immediately (not just the current step): the post's status
    /// becomes `rejected`. `comment` is optional (pass `None`; max 2000
    /// characters) and, when given, is shown to the requester and other
    /// approvers in the post's review thread. To name the step you
    /// reviewed, use [`Self::reject_with`].
    pub async fn reject(&self, id: &str, comment: Option<&str>) -> Result<Value, Error> {
        self.reject_with(
            id,
            RejectPostParams {
                comment: comment.map(str::to_string),
                step_order: None,
            },
        )
        .await
    }

    /// `POST /posts/:id/reject` with an optional `comment` and `step_order`
    /// (the `current_step` of [`Self::get_approval`]). When the review moved
    /// on, nothing is rejected and the call returns [`Error::Api`] with
    /// status 409, code `step_changed` and `current_step_order` in its body.
    pub async fn reject_with(&self, id: &str, params: RejectPostParams) -> Result<Value, Error> {
        let path = format!("/posts/{}/reject", encode_path_segment(id));
        if params.comment.is_none() && params.step_order.is_none() {
            return self.client.post_empty(&path).await;
        }
        self.client.post_json(&path, &params).await
    }

    /// `GET /posts/:id/approval` - the approval review of a post: every
    /// step with its approvers and their decisions, the rejection with its
    /// reason, and the comment thread. Use it when `approval_status` is
    /// `rejected` to learn who rejected the post and why, or while it is
    /// `pending` to see who the post waits for.
    ///
    /// `data` carries `post_id`, `status` (`none`, `pending`, `approved`,
    /// `rejected`), `workflow`, `requested_by`, `requested_at`,
    /// `current_step`, `steps`, `rejection` and `comments` (oldest first).
    /// A post without an approval workflow returns `status: "none"` with
    /// the object fields `null` and empty `steps` and `comments`.
    /// Read-only; requires the `posts:read` scope.
    pub async fn get_approval(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/posts/{}/approval", encode_path_segment(id)), Vec::new())
            .await
    }
}

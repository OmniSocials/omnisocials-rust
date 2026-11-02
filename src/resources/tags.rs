use serde_json::Value;

use crate::client::Client;
use crate::error::Error;
use crate::tag_params::{ApplyTagsParams, CreateTagParams, TagAnalyticsParams, UpdateTagParams};

use super::Analytics;

/// `client.tags()`: post tags, the workspace's own labels for its posts (a
/// hook style, a format) to filter and compare posts in Analytics. Tags
/// never go to a platform. Set them on a post with
/// [`crate::CreatePostParams::tags`] / [`crate::UpdatePostParams::tags`],
/// filter with [`crate::ListPostsParams::tag`] and
/// [`crate::AnalyticsOverviewParams::tag`], compare them with
/// [`Analytics::tags`]. Uses the `posts:read` / `posts:write` scopes.
///
/// Error codes: 404 `tag_not_found`; 409 `tag_exists` (the existing tag is
/// in the error body under `error.tag`), `tag_limit_reached` (200 tags per workspace); 400 `invalid_tag_name`
/// (1 to 40 characters), `invalid_tag_color`, `invalid_tags`,
/// `invalid_post_ids`, `too_many_posts` (500 per apply), `too_many_tags`
/// (20 per post).
#[derive(Debug, Clone, Copy)]
pub struct Tags<'a> {
    pub(crate) client: &'a Client,
}

impl Tags<'_> {
    /// `GET /tags` - every tag of the workspace, by name:
    /// `{"data": [{"id", "name", "color", "created_at"}]}`.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/tags", Vec::new()).await
    }

    /// `POST /tags` - create a tag (at most 200 per workspace). A name that
    /// exists answers 409 `tag_exists`.
    pub async fn create(&self, params: CreateTagParams) -> Result<Value, Error> {
        self.client.post_json("/tags", &params).await
    }

    /// `PATCH /tags/:id` - rename or recolour a tag. Posts keep it.
    pub async fn update(&self, id: i64, params: UpdateTagParams) -> Result<Value, Error> {
        self.client.patch_json(&format!("/tags/{id}"), &params).await
    }

    /// `DELETE /tags/:id` - delete the tag and take it off every post of the
    /// workspace; the posts stay. Returns
    /// `{"data": {"tag": {...}, "posts_untagged": n}}`.
    pub async fn delete(&self, id: i64) -> Result<Value, Error> {
        self.client.delete(&format!("/tags/{id}")).await
    }

    /// `POST /tags/apply` - add, remove or replace the tags of up to 500
    /// posts. Returns `{"data": {"updated", "posts": [{"id", "tags"}],
    /// "not_found"}}`.
    pub async fn apply(&self, params: ApplyTagsParams) -> Result<Value, Error> {
        self.client.post_json("/tags/apply", &params).await
    }
}

impl Analytics<'_> {
    /// `GET /analytics/tags` - how each tag did against your normal for the
    /// published posts in a period. `data` = `{ period, platform, min_posts,
    /// tags: [{ tag, posts, measured, median_impressions, engagement_rate,
    /// vs_normal, best_channel, enough_posts }], untagged, normals }`; tags
    /// come comparable first, best first. `vs_normal` is a fraction (0.38 =
    /// 38% above), `engagement_rate` a percent. Needs `analytics:read`.
    pub async fn tags(&self, params: TagAnalyticsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(start_date) = params.start_date {
            query.push(("start_date", start_date));
        }
        if let Some(end_date) = params.end_date {
            query.push(("end_date", end_date));
        }
        if let Some(platform) = params.platform {
            query.push(("platform", platform));
        }
        if let Some(min_posts) = params.min_posts {
            query.push(("min_posts", min_posts.to_string()));
        }
        self.client.get("/analytics/tags", query).await
    }
}

use serde_json::Value;

use crate::client::{encode_path_segment, Client, Retry};
use crate::error::Error;
use crate::types::{AccountAnalyticsParams, AnalyticsOverviewParams, BestTimesParams};

/// `client.analytics()`: post, account, and workspace analytics.
#[derive(Debug, Clone, Copy)]
pub struct Analytics<'a> {
    pub(crate) client: &'a Client,
}

impl Analytics<'_> {
    /// `GET /analytics/posts/:id` - latest per-platform metrics for one post.
    pub async fn post(&self, post_id: &str) -> Result<Value, Error> {
        self.client
            .get(
                &format!("/analytics/posts/{}", encode_path_segment(post_id)),
                Vec::new(),
            )
            .await
    }

    /// `POST /analytics/posts/:id/refresh` - "Update now": read the X numbers
    /// of one published post now, outside its check schedule. X bills every
    /// read, so it costs half a credit per tweet (each part of a thread
    /// counts, plus half a credit for a video or GIF), and nothing when the
    /// post was already read today (UTC). OmniSocials Connect accounts pay on
    /// the partner's own X app. At most 5 reads of a post a UTC day. Needs
    /// `analytics:read`. Returns `{"data": {"post_id", "tweets": [{"id",
    /// "metrics", "read_at"}], "credits_used", "read_at"}}`. Because it can
    /// cost credits it is retried only after a 429, never after a 5xx, a
    /// timeout or a dropped connection. Errors: 402 `insufficient_credits`,
    /// 404 `not_found`, 409 `refresh_not_available` (no published X tweet),
    /// 429 `refresh_limit`, 502 `refresh_failed` (nothing charged).
    pub async fn refresh_post(&self, post_id: &str) -> Result<Value, Error> {
        self.client
            .post_empty_with(
                &format!("/analytics/posts/{}/refresh", encode_path_segment(post_id)),
                Retry::RateLimit,
            )
            .await
    }

    /// `GET /analytics/posts?ids=a,b,c` - batch metrics for up to 100 posts
    /// in one call instead of one request per post.
    pub async fn posts<S: AsRef<str>>(&self, post_ids: &[S]) -> Result<Value, Error> {
        let ids = post_ids
            .iter()
            .map(|id| id.as_ref())
            .collect::<Vec<_>>()
            .join(",");
        self.client.get("/analytics/posts", vec![("ids", ids)]).await
    }

    /// `GET /analytics/overview` - workspace-wide totals for a period.
    pub async fn overview(&self, params: AnalyticsOverviewParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(period) = params.period {
            query.push(("period", period));
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
        self.client.get("/analytics/overview", query).await
    }

    /// `GET /analytics/accounts` - account-level stats (followers etc).
    pub async fn accounts(&self, params: AccountAnalyticsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(platform) = params.platform {
            query.push(("platform", platform));
        }
        if let Some(date) = params.date {
            query.push(("date", date));
        }
        self.client.get("/analytics/accounts", query).await
    }

    /// `GET /analytics/best-times` - recommended posting time slots for a
    /// platform, based on when the account's audience engages.
    pub async fn best_times(&self, params: BestTimesParams) -> Result<Value, Error> {
        let mut query = vec![("platform", params.platform)];
        if let Some(timezone) = params.timezone {
            query.push(("timezone", timezone));
        }
        self.client.get("/analytics/best-times", query).await
    }
}

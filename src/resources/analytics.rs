use serde_json::Value;

use crate::client::{encode_path_segment, Client};
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

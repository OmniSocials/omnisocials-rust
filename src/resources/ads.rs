use reqwest::Method;
use serde_json::{json, Value};

use crate::client::{encode_path_segment, Client, Retry};
use crate::error::Error;
use crate::types::{
    AdDraftParams, AttachAdDraftAccountParams, BoostPostMultiParams, BoostPostParams,
    CreateAdCampaignParams, KeywordMetricsParams, KeywordResearchParams, LaunchAdDraftParams,
    ListAdCampaignsParams, UpdateAdCampaignParams,
};

/// `client.ads()`: boost a published post or create a new ad on Meta,
/// Pinterest, TikTok, LinkedIn, X and Google.
///
/// Ads spend real money once a campaign starts. Boosts, campaigns and drafts
/// are created as drafts (no spend). [`activate`](Ads::activate),
/// [`activate_multi`](Ads::activate_multi), [`AdDrafts::launch`] and a
/// budget raise through [`update`](Ads::update) start or raise spend, and
/// each needs the amount again as a confirmation. Show the person the amount
/// and get their yes first. These calls and [`resume`](Ads::resume) are not
/// retried after a 5xx or a connection error (only a 429 is), because the
/// first attempt may have started the ad: read the campaign or the draft to
/// see what happened.
///
/// Budgets are in major currency units (10 = 10 EUR) in the ad account's
/// currency. Ad accounts are connected and picked in the OmniSocials
/// dashboard. The scopes are opt-in on the API key (the "Ads" switch):
/// `ads:read` for the reads, `ads:write` for the rest; a key without them
/// gets `Error::PermissionDenied` with code `insufficient_scope`.
#[derive(Debug, Clone, Copy)]
pub struct Ads<'a> {
    pub(crate) client: &'a Client,
}

/// `client.ads().drafts()`: "Create an ad" drafts, one campaign per network
/// in `platforms`, each with the same ads. A draft never spends;
/// [`launch`](AdDrafts::launch) starts it.
#[derive(Debug, Clone, Copy)]
pub struct AdDrafts<'a> {
    pub(crate) client: &'a Client,
}

fn campaign_path(id: &str) -> String {
    format!("/ads/campaigns/{}", encode_path_segment(id))
}

fn draft_path(group_id: &str) -> String {
    format!("/ads/drafts/{}", encode_path_segment(group_id))
}

impl<'a> Ads<'a> {
    /// "Create an ad" drafts: defaults, create, get, update, delete,
    /// attach_account, launch.
    pub fn drafts(&self) -> AdDrafts<'a> {
        AdDrafts { client: self.client }
    }

    /// `GET /ads/accounts` - the connected ad accounts, plus
    /// `platforms_available`. Accounts are connected and picked in the
    /// dashboard; only `is_selected` accounts can run campaigns. Requires
    /// `ads:read`.
    pub async fn list_accounts(&self) -> Result<Value, Error> {
        self.client.get("/ads/accounts", Vec::new()).await
    }

    /// `GET /ads/capabilities` - what each network accepts for a new ad
    /// (goals, media, text limits, buttons, minimum daily budget),
    /// `create_networks` and `network_states` (`live`, `preview` or
    /// `coming_soon`). Requires `ads:read`.
    pub async fn capabilities(&self) -> Result<Value, Error> {
        self.client.get("/ads/capabilities", Vec::new()).await
    }

    /// `GET /ads/campaigns` - list campaigns and boosts. Requires `ads:read`.
    pub async fn list(&self, params: ListAdCampaignsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(status) = params.status {
            query.push(("status", status));
        }
        if let Some(platform) = params.platform {
            query.push(("platform", platform));
        }
        if let Some(campaign_type) = params.campaign_type {
            query.push(("campaign_type", campaign_type));
        }
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = params.offset {
            query.push(("offset", offset.to_string()));
        }
        self.client.get("/ads/campaigns", query).await
    }

    /// `GET /ads/campaigns/:id` - one campaign with its latest numbers.
    /// Requires `ads:read`.
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client.get(&campaign_path(id), Vec::new()).await
    }

    /// `POST /ads/boost` - boost a published post. Creates a DRAFT boost:
    /// no spend until [`activate`](Ads::activate). TikTok needs
    /// `creative.spark_auth_code`, Google needs `headlines`, `descriptions`
    /// and `logo_url`. Requires `ads:write`.
    pub async fn boost(&self, params: BoostPostParams) -> Result<Value, Error> {
        self.client.post_json("/ads/boost", &params).await
    }

    /// `POST /ads/boost-multi` - one DRAFT boost per ad account (up to 10),
    /// each with the same budget, so the total is amount x accounts.
    /// Answers `{"results": [...]}` (no `data`); one failed account never
    /// stops the others. Requires `ads:write`.
    pub async fn boost_multi(&self, params: BoostPostMultiParams) -> Result<Value, Error> {
        self.client.post_json("/ads/boost-multi", &params).await
    }

    /// `POST /ads/campaigns` - a DRAFT campaign without ads. It cannot
    /// start (`campaign_has_no_ads`): use [`AdDrafts::create`] for a new
    /// ad. Requires `ads:write`.
    pub async fn create(&self, params: CreateAdCampaignParams) -> Result<Value, Error> {
        self.client.post_json("/ads/campaigns", &params).await
    }

    /// `PATCH /ads/campaigns/:id` - change the name, budget, targeting or
    /// schedule. To RAISE the budget of a running campaign, which raises
    /// real ad spend, show the person the new amount, get their yes, and
    /// pass it again as `confirm_budget_amount`. Not retried after a 5xx or
    /// a connection error: read the campaign with [`get`](Ads::get) to see
    /// what happened. Requires `ads:write`.
    pub async fn update(&self, id: &str, params: UpdateAdCampaignParams) -> Result<Value, Error> {
        self.client
            .send_json(Method::PATCH, &campaign_path(id), &params, Retry::RateLimit)
            .await
    }

    /// `POST /ads/campaigns/:id/activate` - START REAL AD SPEND. Show the
    /// person the budget (amount, currency, daily or lifetime) and get
    /// their yes first; `confirm_budget_amount` must equal the campaign's
    /// budget amount. A campaign of a draft group answers
    /// `409 use_draft_launch`: use [`AdDrafts::launch`]. Not retried after a
    /// 5xx or a connection error: read the campaign with [`get`](Ads::get)
    /// to see what happened. Requires `ads:write`.
    pub async fn activate(&self, id: &str, confirm_budget_amount: f64) -> Result<Value, Error> {
        let body = json!({ "confirm_budget_amount": confirm_budget_amount });
        self.client
            .send_json(
                Method::POST,
                &format!("{}/activate", campaign_path(id)),
                &body,
                Retry::RateLimit,
            )
            .await
    }

    /// `POST /ads/campaigns/activate-multi` - START REAL AD SPEND on up to
    /// 10 campaigns. Every campaign's budget must equal
    /// `confirm_budget_amount`; show the person the total (amount x
    /// campaigns) and get their yes first. Answers `{"results": [...]}`.
    /// Not retried after a 5xx or a connection error: read the campaigns to
    /// see what happened. Requires `ads:write`.
    pub async fn activate_multi<S: AsRef<str>>(
        &self,
        campaign_ids: &[S],
        confirm_budget_amount: f64,
    ) -> Result<Value, Error> {
        let ids: Vec<&str> = campaign_ids.iter().map(AsRef::as_ref).collect();
        let body = json!({
            "campaign_ids": ids,
            "confirm_budget_amount": confirm_budget_amount,
        });
        self.client
            .send_json(Method::POST, "/ads/campaigns/activate-multi", &body, Retry::RateLimit)
            .await
    }

    /// `POST /ads/campaigns/:id/pause` - pause a campaign (also while in
    /// review). Requires `ads:write`.
    pub async fn pause(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("{}/pause", campaign_path(id)))
            .await
    }

    /// `POST /ads/campaigns/:id/resume` - resume a paused campaign, which
    /// STARTS REAL AD SPEND again. Show the person the budget and get their
    /// yes first. Not retried after a 5xx or a connection error: read the
    /// campaign with [`get`](Ads::get) to see what happened. Requires
    /// `ads:write`.
    pub async fn resume(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty_with(&format!("{}/resume", campaign_path(id)), Retry::RateLimit)
            .await
    }

    /// `DELETE /ads/campaigns/:id` - delete a campaign, on the network too.
    /// A campaign that spent is archived: its spend stays in the totals.
    /// Answers `{"data": {"deleted": true}}`. Requires `ads:write`.
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client.delete(&campaign_path(id)).await
    }

    /// `GET /ads/overview` - accounts, campaigns per status and total
    /// spend. Requires `ads:read`.
    pub async fn overview(&self) -> Result<Value, Error> {
        self.client.get("/ads/overview", Vec::new()).await
    }

    /// `GET /ads/suggestions` - organic posts that do better than the
    /// workspace median: boost candidates. Read-only. Requires `ads:read`.
    pub async fn suggestions(&self) -> Result<Value, Error> {
        self.client.get("/ads/suggestions", Vec::new()).await
    }

    /// `POST /ads/research/keywords` - keyword ideas from seed keywords or a
    /// URL, through the connected Google Ads account. Read-only and free.
    /// Requires `ads:read`.
    pub async fn research_keywords(&self, params: KeywordResearchParams) -> Result<Value, Error> {
        self.client.post_json("/ads/research/keywords", &params).await
    }

    /// `POST /ads/research/keywords/metrics` - search numbers for an exact
    /// keyword list. Read-only. Requires `ads:read`.
    pub async fn keyword_metrics(&self, params: KeywordMetricsParams) -> Result<Value, Error> {
        self.client
            .post_json("/ads/research/keywords/metrics", &params)
            .await
    }
}

impl AdDrafts<'_> {
    /// `GET /ads/drafts/defaults` - suggested countries (where the Instagram
    /// followers are), the currency and the workspace's ad account per
    /// network. Requires `ads:read`.
    pub async fn defaults(&self) -> Result<Value, Error> {
        self.client.get("/ads/drafts/defaults", Vec::new()).await
    }

    /// `POST /ads/drafts` - save a new ad as a draft group. It needs no ad
    /// account and spends nothing. Read [`Ads::capabilities`] first for what
    /// each network accepts. The answer carries `review_url` (the builder in
    /// the dashboard) and per network `readiness`. Requires `ads:write`.
    pub async fn create(&self, params: AdDraftParams) -> Result<Value, Error> {
        self.client.post_json("/ads/drafts", &params).await
    }

    /// `GET /ads/drafts/:group_id` - one draft group. Requires `ads:read`.
    pub async fn get(&self, group_id: &str) -> Result<Value, Error> {
        self.client.get(&draft_path(group_id), Vec::new()).await
    }

    /// `PUT /ads/drafts/:group_id` - replace the draft with the full body.
    /// Networks that already started are left alone (`campaigns[].editable`).
    /// Requires `ads:write`.
    pub async fn update(&self, group_id: &str, params: AdDraftParams) -> Result<Value, Error> {
        self.client.put_json(&draft_path(group_id), &params).await
    }

    /// `DELETE /ads/drafts/:group_id` - delete a draft group that never
    /// started. Requires `ads:write`.
    pub async fn delete(&self, group_id: &str) -> Result<Value, Error> {
        self.client.delete(&draft_path(group_id)).await
    }

    /// `POST /ads/drafts/:group_id/account` - set the ad account of one
    /// network in the draft (from [`Ads::list_accounts`], the same network).
    /// Requires `ads:write`.
    pub async fn attach_account(
        &self,
        group_id: &str,
        params: AttachAdDraftAccountParams,
    ) -> Result<Value, Error> {
        self.client
            .post_json(&format!("{}/account", draft_path(group_id)), &params)
            .await
    }

    /// `POST /ads/drafts/:group_id/launch` - START REAL AD SPEND for every
    /// network of the draft that never started. Show the person the daily
    /// amount per network and get their yes first, then pass it as
    /// `confirm_budget_amount` (or per campaign as
    /// `confirm_budget_amounts`). A wrong or missing amount answers
    /// `400 budget_confirmation_required` and writes nothing. Answers
    /// `{"data": {"group": {...}, "results": [...]}}`, one result per
    /// network; one failed network never stops the others. Not retried
    /// after a 5xx or a connection error: read the draft with
    /// [`get`](AdDrafts::get) to see what started. Requires `ads:write`.
    pub async fn launch(&self, group_id: &str, params: LaunchAdDraftParams) -> Result<Value, Error> {
        self.client
            .send_json(
                Method::POST,
                &format!("{}/launch", draft_path(group_id)),
                &params,
                Retry::RateLimit,
            )
            .await
    }
}

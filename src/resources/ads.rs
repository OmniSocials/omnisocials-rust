use reqwest::Method;
use serde::Serialize;
use serde_json::{json, Value};

use super::ads_parity::{
    post_maybe_empty, AdAccounts, AdAudiences, AdBudgetSchedules, AdCampaignAds, AdComments,
    AdConversions, AdGoogle, AdInsights, AdLeadForms, AdLeads, AdMedia, AdObjects, AdPartnership,
    AdPreviews, AdReachFrequency, AdReports, AdResults, AdRules, AdStudies, AdTargeting,
};
use crate::client::{encode_path_segment, Client, Retry};
use crate::error::Error;
use crate::types::{
    AdDraftParams, AttachAdDraftAccountParams, BoostPostMultiParams, BoostPostParams,
    KeywordMetricsParams, KeywordResearchParams, LaunchAdDraftParams, ListAdCampaignsParams,
    UpdateAdCampaignParams,
};

/// `client.ads()`: boost a published post or create a new ad on Meta,
/// Pinterest, TikTok, LinkedIn, X and Google.
///
/// Ads spend real money once a campaign starts. Boosts and drafts are
/// created as drafts (no spend). Starting or raising spend needs a key with
/// the opt-in `ads:spend` scope ("Start ads", a second switch under "Ads";
/// its creator needs `manage_ads`): [`activate`](Ads::activate),
/// [`activate_multi`](Ads::activate_multi), [`resume`](Ads::resume),
/// [`AdDrafts::launch`], a budget raise or any change of a campaign that
/// reached the network through [`update`](Ads::update), and the parity calls
/// that switch something on or raise a budget. Each repeats the amount: show
/// the person the amount and get their yes first. Without `ads:spend` these
/// calls answer `403 start_in_app` with the page in the app where a person
/// does it; on Google (`api_draft_only` of
/// [`capabilities`](Ads::capabilities)) they always do, also with
/// `ads:spend`. [`pause`](Ads::pause) and [`delete`](Ads::delete) stay
/// open, except on the networks in `api_stop_in_app` (Google).
///
/// The calls that start or raise spend, and [`resume`](Ads::resume), are not
/// retried after a 5xx or a connection error (only a 429 is), because the
/// first attempt may have started the ad: read the campaign or the draft to
/// see what happened. The same holds for [`bulk_status`](Ads::bulk_status)
/// and [`AdCampaignAds::set_status`] (switching on),
/// [`AdObjects::update`] and [`AdObjects::set_status`], [`AdRules::create`],
/// [`AdRules::update`] and [`AdRules::undo`], [`AdAccounts::update_settings`]
/// (a higher spend cap), [`AdReachFrequency::reserve`] and
/// [`AdBudgetSchedules::create`]. Take the amount from the person's explicit
/// yes; never invent or confirm it yourself. Pausing never needs an amount.
/// Ads errors may carry `param` (the field at fault): read it from the
/// error's `body` (`body["error"]["param"]`).
///
/// Budgets are in major currency units (10 = 10 EUR) in the ad account's
/// currency. Ad accounts are connected and picked in the OmniSocials
/// dashboard. The scopes are opt-in on the API key (the "Ads" switch):
/// `ads:read` for the reads, `ads:write` for the rest; a key without them
/// gets `Error::PermissionDenied` with code `insufficient_scope`, and every
/// write also needs `manage_ads` for the person behind the key. `ads:spend`
/// (the "Start ads" switch under it) is only for the calls that start or
/// raise spend (see above). A network
/// without a feature answers `400 not_supported_on_network`;
/// [`capabilities`](Ads::capabilities) lists `features` per network.
#[derive(Debug, Clone, Copy)]
pub struct Ads<'a> {
    pub(crate) client: &'a Client,
}

/// `client.ads().drafts()`: "Create an ad" drafts, one campaign per network
/// in `platforms`, each with the same ads. A draft never spends;
/// [`launch`](AdDrafts::launch) starts it (a key with `ads:spend`), or a
/// person does in the app from its `review_url`.
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

    /// Campaigns, ad sets and ads as the network has them, and the import of
    /// campaigns made outside OmniSocials.
    pub fn objects(&self) -> AdObjects<'a> {
        AdObjects { client: self.client }
    }

    /// Targeting search, browse, reach estimate and bid suggestions.
    pub fn targeting(&self) -> AdTargeting<'a> {
        AdTargeting { client: self.client }
    }

    /// Custom audiences: customer lists, lookalikes, company lists.
    pub fn audiences(&self) -> AdAudiences<'a> {
        AdAudiences { client: self.client }
    }

    /// The ads of an OmniSocials campaign: list, add, read, status, review.
    pub fn campaign_ads(&self) -> AdCampaignAds<'a> {
        AdCampaignAds { client: self.client }
    }

    /// Lead forms.
    pub fn lead_forms(&self) -> AdLeadForms<'a> {
        AdLeadForms { client: self.client }
    }

    /// The leads the lead forms bring.
    pub fn leads(&self) -> AdLeads<'a> {
        AdLeads { client: self.client }
    }

    /// Pixels, conversion goals, custom conversions and server events.
    pub fn conversions(&self) -> AdConversions<'a> {
        AdConversions { client: self.client }
    }

    /// Live numbers, campaign and ad numbers, timeline, rankings and
    /// alerts.
    pub fn insights(&self) -> AdInsights<'a> {
        AdInsights { client: self.client }
    }

    /// Long reports that run in the background.
    pub fn reports(&self) -> AdReports<'a> {
        AdReports { client: self.client }
    }

    /// Automated rules.
    pub fn rules(&self) -> AdRules<'a> {
        AdRules { client: self.client }
    }

    /// Your own numbers per campaign and day.
    pub fn results(&self) -> AdResults<'a> {
        AdResults { client: self.client }
    }

    /// The network's own image and video library.
    pub fn media(&self) -> AdMedia<'a> {
        AdMedia { client: self.client }
    }

    /// Previews drawn by the network.
    pub fn previews(&self) -> AdPreviews<'a> {
        AdPreviews { client: self.client }
    }

    /// Partnership ads with creators (Meta).
    pub fn partnership(&self) -> AdPartnership<'a> {
        AdPartnership { client: self.client }
    }

    /// Comments on ads.
    pub fn comments(&self) -> AdComments<'a> {
        AdComments { client: self.client }
    }

    /// One ad account: health, finance, settings, people, activity,
    /// identities.
    pub fn accounts(&self) -> AdAccounts<'a> {
        AdAccounts { client: self.client }
    }

    /// Reach and frequency buying (Meta).
    pub fn reach_frequency(&self) -> AdReachFrequency<'a> {
        AdReachFrequency { client: self.client }
    }

    /// A/B tests and lift studies (Meta).
    pub fn studies(&self) -> AdStudies<'a> {
        AdStudies { client: self.client }
    }

    /// Planned budget increases (Meta).
    pub fn budget_schedules(&self) -> AdBudgetSchedules<'a> {
        AdBudgetSchedules { client: self.client }
    }

    /// Google Ads reports.
    pub fn google(&self) -> AdGoogle<'a> {
        AdGoogle { client: self.client }
    }

    /// `GET /ads/accounts` - the connected ad accounts, plus
    /// `platforms_available`. Accounts are connected and picked in the
    /// dashboard; only `is_selected` accounts can run campaigns. Requires
    /// `ads:read`.
    pub async fn list_accounts(&self) -> Result<Value, Error> {
        self.client.get("/ads/accounts", Vec::new()).await
    }

    /// `GET /ads/capabilities` - what each network accepts for a new ad
    /// (goals, media, text limits, buttons, minimum daily budget, and per
    /// network `bid_strategies`, `settings` and `ad_fields`),
    /// `create_networks`, `network_states` (`live`, `preview` or
    /// `coming_soon`), `features` (per network, the feature keys its API
    /// offers here) with `feature_labels`, `targeting_types` (per network,
    /// the types of `targeting().search` and of `targeting.networks`),
    /// `api_goals` (`lead_generation`, `conversions`, `messages`, `calls`:
    /// API only; each network's goal entry says what it `needs`),
    /// `api_draft_only` (networks where every API caller stops at the draft,
    /// also a key with `ads:spend`: Google) and `api_stop_in_app` (networks
    /// where the API may not even pause or delete a campaign that reached
    /// the network: Google). Requires `ads:read`.
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

    /// `POST /ads/boost` - boost a published post. Creates a DRAFT boost
    /// that spends nothing; the person starts it in the app. TikTok needs
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

    /// `PATCH /ads/campaigns/:id` - change the name, budget, targeting or
    /// schedule of a draft. A campaign that reached the network changes only
    /// with a key that has `ads:spend`; without it, and on Google
    /// (`api_draft_only`) always, the call answers `403 start_in_app` and a
    /// person changes it in the app. `confirm_budget_amount` (the new amount
    /// again, after the person agreed to it) is for a budget raise of a
    /// running campaign. Not retried after a 5xx or a connection error: read
    /// the campaign with [`get`](Ads::get) to see what happened. Requires
    /// `ads:write`.
    pub async fn update(&self, id: &str, params: UpdateAdCampaignParams) -> Result<Value, Error> {
        self.client
            .send_json(Method::PATCH, &campaign_path(id), &params, Retry::RateLimit)
            .await
    }

    /// `POST /ads/campaigns/:id/activate` - START REAL AD SPEND. Needs a key
    /// with the opt-in `ads:spend` scope ("Start ads"); without it, and on
    /// Google (`api_draft_only`) always, the call answers `403 start_in_app`
    /// with the page where a person does it in the app. Show the person the
    /// budget (amount, currency, daily or lifetime) and get their yes first;
    /// `confirm_budget_amount` must equal the campaign's budget amount. A
    /// campaign of a draft group answers `409 use_draft_launch`: use
    /// [`AdDrafts::launch`]. Not retried after a 5xx or a connection error:
    /// read the campaign with [`get`](Ads::get) to see what happened.
    /// Requires `ads:write`.
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

    /// `POST /ads/campaigns/activate-multi` - START REAL AD SPEND on up to 10
    /// campaigns. Needs a key with `ads:spend`: without it, and on Google
    /// (`api_draft_only`) always, a result has `ok: false` and `code`
    /// `start_in_app` with the page where a person starts it. Every campaign's
    /// budget must equal `confirm_budget_amount`; show the person the total
    /// (amount x campaigns) and get their yes first. Answers `{"results":
    /// [...]}`. Not retried after a 5xx or a connection error: read the
    /// campaigns to see what happened. Requires `ads:write`.
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
    /// review). Open through the API, except on the networks in
    /// `api_stop_in_app` (Google): there a campaign that reached the network
    /// answers `403 start_in_app`. Requires `ads:write`.
    pub async fn pause(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("{}/pause", campaign_path(id)))
            .await
    }

    /// `POST /ads/campaigns/:id/resume` - resume a paused campaign, which
    /// STARTS REAL AD SPEND again. Needs a key with the opt-in `ads:spend`
    /// scope ("Start ads"); without it, and on Google (`api_draft_only`)
    /// always, the call answers `403 start_in_app` with the page where a
    /// person does it in the app. Show the person the budget and get their
    /// yes first. Only a paused campaign resumes (`409 not_paused`). Not
    /// retried after a 5xx or a connection error: read the campaign with
    /// [`get`](Ads::get) to see what happened. Requires `ads:write`.
    pub async fn resume(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty_with(&format!("{}/resume", campaign_path(id)), Retry::RateLimit)
            .await
    }

    /// `DELETE /ads/campaigns/:id` - delete a campaign, on the network too.
    /// A campaign that spent is archived: its spend stays in the totals.
    /// Open through the API, except a campaign that reached the network on a
    /// network in `api_stop_in_app` (Google): `403 start_in_app`. Answers
    /// `{"data": {"deleted": true}}`. Requires `ads:write`.
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client.delete(&campaign_path(id)).await
    }

    /// `GET /ads/overview` - accounts, campaigns per status and the totals.
    /// `totals.spend` is `null` when the campaigns use more than one
    /// currency: read `totals_by_currency` (`currency`, `spend`,
    /// `impressions`, `clicks` per currency) then. Requires `ads:read`.
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

    /// `POST /ads/campaigns/bulk-status` - pause (`"paused"`) or resume
    /// (`"active"`) up to 50 OmniSocials campaigns. Resuming STARTS REAL AD
    /// SPEND again: every campaign's budget must equal
    /// `confirm_budget_amount` (one amount for all, checked before any
    /// change); show the person the total and get their yes first. Resuming
    /// needs a key with `ads:spend`: without it, and on Google always, a
    /// result answers `start_in_app`. Pausing needs `None`. Answers
    /// `results` per campaign. Not retried after a 5xx
    /// or a connection error: read the campaigns with [`get`](Ads::get).
    /// Requires `ads:write`.
    pub async fn bulk_status<S: AsRef<str>>(
        &self,
        campaign_ids: &[S],
        status: &str,
        confirm_budget_amount: Option<f64>,
    ) -> Result<Value, Error> {
        let ids: Vec<&str> = campaign_ids.iter().map(AsRef::as_ref).collect();
        let mut body = json!({ "campaign_ids": ids, "status": status });
        if let Some(amount) = confirm_budget_amount {
            body["confirm_budget_amount"] = json!(amount);
        }
        self.client
            .send_json(Method::POST, "/ads/campaigns/bulk-status", &body, Retry::RateLimit)
            .await
    }

    /// `POST /ads/campaigns/:id/duplicate` - copy an OmniSocials campaign as
    /// a new DRAFT (same ad account, goal, budget, targeting, settings and
    /// ads; optional `name`, or `()`). A draft never spends. Requires
    /// `ads:write`.
    pub async fn duplicate<B: Serialize>(&self, id: &str, params: B) -> Result<Value, Error> {
        post_maybe_empty(self.client, &format!("{}/duplicate", campaign_path(id)), &params).await
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
    /// the dashboard, where the person checks and starts the ad: hand it
    /// over) and per network `readiness`. A `budget.currency` other than the
    /// attached ad account's currency answers `400 currency_mismatch` with
    /// `param: "budget.currency"`. Requires `ads:write`.
    pub async fn create(&self, params: AdDraftParams) -> Result<Value, Error> {
        self.client.post_json("/ads/drafts", &params).await
    }

    /// `GET /ads/drafts/:group_id` - one draft group. Requires `ads:read`.
    pub async fn get(&self, group_id: &str) -> Result<Value, Error> {
        self.client.get(&draft_path(group_id), Vec::new()).await
    }

    /// `PUT /ads/drafts/:group_id` - replace the draft with the full body.
    /// Networks that already started are left alone (`campaigns[].editable`).
    /// A `budget.currency` other than the attached ad account's currency
    /// answers `400 currency_mismatch` with `param: "budget.currency"`.
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
    /// network of the draft that never started. Needs a key with the opt-in
    /// `ads:spend` scope; without it the call answers `403 start_in_app`
    /// with the page where a person starts it (the draft's `review_url`),
    /// and a network in `api_draft_only` (Google) is always skipped with
    /// code `start_in_app`. Show the person the daily amount per network and
    /// get their yes first, then pass it as `confirm_budget_amount` (or per
    /// campaign as `confirm_budget_amounts`). A wrong or missing amount
    /// answers `400 budget_confirmation_required` and writes nothing.
    /// Answers `{"data": {"group": {...}, "results": [...]}}`, one result
    /// per network (`campaign_id` is a string); one failed network never
    /// stops the others. Not retried after a 5xx or a connection error: read
    /// the draft with [`get`](AdDrafts::get) to see what started. Requires
    /// `ads:write`.
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

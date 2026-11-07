//! The Ads API parity groups (2026-11-05), reached from `client.ads()`.
//!
//! A thin passthrough: a query is anything that serializes to a JSON object
//! (`json!({"from": "2026-10-01", "limit": 50})`; a list becomes a comma
//! list, `null` values are left out; pass `()` for none) and a body anything
//! that serializes to JSON. The API checks the values. A network without a
//! feature answers `400 not_supported_on_network` (`capabilities` lists
//! `features`). A missing permission on the network side answers
//! `403 network_permission_missing`.
//!
//! Money moments (switching something on, a budget or spend cap change, a
//! rule that can raise spend, a reach and frequency reservation, a study, a
//! planned budget increase) and other changes of a campaign on the network
//! need a key with the opt-in `ads:spend` scope ("Start ads"): without it,
//! and on Google always, they answer `403 start_in_app` with the page where
//! a person does it in the app (rules: `403 rule_raises_spend`). Pausing,
//! deleting and archiving stay open, except on Google. With `ads:spend` a
//! money moment answers `400 budget_confirmation_required` until the
//! person's own amount comes back as `confirm_budget_amount` (rules:
//! `confirm_max_daily_budget`). Calls that can start or raise ad spend are
//! retried after a 429 only, never after a 5xx or a connection error: take
//! the amount from the person's explicit yes, never invent or confirm it
//! yourself.

use reqwest::Method;
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::client::{encode_path_segment, Client, Retry};
use crate::error::Error;

fn serialization_error(err: serde_json::Error) -> Error {
    Error::Validation {
        status: 400,
        code: Some("serialization_error".into()),
        message: format!("Could not serialize the request: {err}"),
        body: None,
    }
}

/// One query value as the API reads it: strings as they are, numbers and
/// booleans as text, a list as a comma list.
fn query_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Array(items) => Some(items.iter().filter_map(query_value).collect::<Vec<_>>().join(",")),
        Value::Object(_) => Some(value.to_string()),
    }
}

/// `path?key=value&...` from a query that serializes to an object (or to
/// `null` / `()` for none). Keys and values are percent-encoded.
pub(crate) fn with_query<Q: Serialize>(path: &str, query: &Q) -> Result<String, Error> {
    let value = serde_json::to_value(query).map_err(serialization_error)?;
    let pairs: Vec<String> = match &value {
        Value::Null => Vec::new(),
        Value::Object(map) => map
            .iter()
            .filter_map(|(key, value)| {
                query_value(value)
                    .map(|v| format!("{}={}", encode_path_segment(key), encode_path_segment(&v)))
            })
            .collect(),
        _ => {
            return Err(Error::Validation {
                status: 400,
                code: Some("serialization_error".into()),
                message: "A query must serialize to a JSON object, for example json!({\"limit\": 50}).".into(),
                body: None,
            })
        }
    };
    Ok(if pairs.is_empty() { path.to_owned() } else { format!("{path}?{}", pairs.join("&")) })
}

fn to_body<B: Serialize>(body: &B) -> Result<Value, Error> {
    serde_json::to_value(body).map_err(serialization_error)
}

/// POST a body that may be empty (`()` or `json!(null)` sends no body).
async fn post_optional<B: Serialize>(client: &Client, path: &str, body: &B, retry: Retry) -> Result<Value, Error> {
    let value = to_body(body)?;
    if value.is_null() {
        client.post_empty_with(path, retry).await
    } else {
        client.send_json(Method::POST, path, &value, retry).await
    }
}

/// `{ status, confirm_budget_amount? }` of the status calls.
pub(crate) fn status_body(status: &str, confirm_budget_amount: Option<f64>) -> Value {
    let mut body = Map::new();
    body.insert("status".into(), json!(status));
    if let Some(amount) = confirm_budget_amount {
        body.insert("confirm_budget_amount".into(), json!(amount));
    }
    Value::Object(body)
}

fn account_path(account_id: &str) -> String {
    format!("/ads/accounts/{}", encode_path_segment(account_id))
}

fn campaign_path(campaign_id: &str) -> String {
    format!("/ads/campaigns/{}", encode_path_segment(campaign_id))
}

fn ad_path(campaign_id: &str, ad_id: &str) -> String {
    format!("{}/ads/{}", campaign_path(campaign_id), encode_path_segment(ad_id))
}

fn account_query(ad_account_id: &str) -> Value {
    json!({ "ad_account_id": ad_account_id })
}

macro_rules! sub_resource {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'a> {
            pub(crate) client: &'a Client,
        }
    };
}

sub_resource!(
    /// `client.ads().objects()`: the campaigns, ad sets and ads of an ad
    /// account as the network has them, also the ones made outside
    /// OmniSocials, and the import of those campaigns. `level` is
    /// `"campaign"`, `"ad_set"` or `"ad"`; `external_id` is the network's own
    /// id. Objects of OmniSocials campaigns answer `409 use_campaign_routes`.
    AdObjects
);
sub_resource!(
    /// `client.ads().targeting()`: targeting search, browse, reach estimate
    /// and bid suggestions. All reads (`ads:read`).
    AdTargeting
);
sub_resource!(
    /// `client.ads().audiences()`: customer lists, lookalikes and company
    /// lists. `audience_id` is the network's own id.
    AdAudiences
);
sub_resource!(
    /// `client.ads().campaign_ads()`: the ads of an OmniSocials campaign
    /// (list, add, read, pause or switch on, review).
    AdCampaignAds
);
sub_resource!(
    /// `client.ads().lead_forms()`: lead forms of an ad account.
    AdLeadForms
);
sub_resource!(
    /// `client.ads().leads()`: the leads the lead forms bring. New leads are
    /// copied every 15 minutes (webhook `lead.received`); OmniSocials stores
    /// the answers encrypted.
    AdLeads
);
sub_resource!(
    /// `client.ads().conversions()`: pixels and datasets, conversion goals,
    /// custom conversions and events from your server.
    AdConversions
);
sub_resource!(
    /// `client.ads().insights()`: live numbers, campaign and ad numbers, the
    /// timeline, rankings and alerts. All reads.
    AdInsights
);
sub_resource!(
    /// `client.ads().reports()`: long reports that run in the background.
    /// All reads.
    AdReports
);
sub_resource!(
    /// `client.ads().rules()`: automated rules that notify, pause, activate
    /// or adjust the budget. A rule that notifies, pauses or lowers a budget
    /// works with `ads:write`. A rule that can raise spend (activate, a
    /// positive adjust_budget) needs a key with `ads:spend` (else
    /// `403 rule_raises_spend`) and `confirm_max_daily_budget` from the
    /// person's yes; it never acts on Google. `create`, `update` and `undo`
    /// are retried after a 429 only.
    AdRules
);
sub_resource!(
    /// `client.ads().results()`: your own numbers per campaign and day
    /// (rules read them as `results.<metric>`).
    AdResults
);
sub_resource!(
    /// `client.ads().media()`: the network's own image and video library.
    /// Use an uploaded file in an ad as `network.<network>.media_ref`.
    AdMedia
);
sub_resource!(
    /// `client.ads().previews()`: previews drawn by the network. Spends
    /// nothing; both calls are reads.
    AdPreviews
);
sub_resource!(
    /// `client.ads().partnership()`: partnership ads with creators (Meta).
    AdPartnership
);
sub_resource!(
    /// `client.ads().comments()`: comments on the ads of an OmniSocials
    /// campaign. `comment_id` is the network's comment id.
    AdComments
);
sub_resource!(
    /// `client.ads().accounts()`: one connected ad account (health, money,
    /// settings, people, the change log, businesses, identities).
    AdAccounts
);
sub_resource!(
    /// `client.ads().reach_frequency()`: reach and frequency buying (Meta).
    /// A prediction spends nothing; a reservation commits the budget.
    AdReachFrequency
);
sub_resource!(
    /// `client.ads().studies()`: A/B tests and lift studies (Meta).
    AdStudies
);
sub_resource!(
    /// `client.ads().budget_schedules()`: planned budget increases for
    /// high-demand periods (Meta).
    AdBudgetSchedules
);
sub_resource!(
    /// `client.ads().google()`: Google Ads reports. Every call takes
    /// `ad_account_id` and is a read; every API caller stops at the draft on
    /// Google (`403 start_in_app`), also a key with `ads:spend`.
    AdGoogle
);

impl AdObjects<'_> {
    fn object_path(account_id: &str, level: &str, external_id: &str) -> String {
        format!(
            "{}/objects/{}/{}",
            account_path(account_id),
            encode_path_segment(level),
            encode_path_segment(external_id)
        )
    }

    /// `GET /ads/accounts/:account_id/tree` - campaigns, ad sets and ads read
    /// live from the network with the numbers of the period. Query: `from`,
    /// `to` (at most 90 days, default the last 30), `status` (a list or a
    /// comma list), `limit` (up to 100), `cursor`. Each campaign says `source`
    /// (`omnisocials` or `external`). Requires `ads:read`.
    pub async fn tree<Q: Serialize>(&self, account_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&format!("{}/tree", account_path(account_id)), &query)?;
        self.client.get(&path, Vec::new()).await
    }

    /// `GET /ads/accounts/:account_id/objects/:level/:external_id` - live
    /// details of one object. Requires `ads:read`.
    pub async fn get(&self, account_id: &str, level: &str, external_id: &str) -> Result<Value, Error> {
        self.client.get(&Self::object_path(account_id, level, external_id), Vec::new()).await
    }

    /// `PATCH /ads/accounts/:account_id/objects/:level/:external_id` - change
    /// the `name`, `budget` or `end_at` of a campaign or ad set made outside
    /// OmniSocials. A higher budget RAISES REAL AD SPEND: show the person the
    /// new amount and get their yes first, then pass it again as
    /// `confirm_budget_amount`. It changes the campaign on the network: needs a
    /// key with the opt-in `ads:spend` scope ("Start ads"); without it, and on
    /// Google (`api_draft_only`) always, the call answers `403 start_in_app`
    /// with the page where a person does it in the app. Not retried after a 5xx
    /// or a connection error: read the object with [`get`](AdObjects::get).
    /// Requires `ads:write`.
    pub async fn update<B: Serialize>(
        &self,
        account_id: &str,
        level: &str,
        external_id: &str,
        params: B,
    ) -> Result<Value, Error> {
        self.client
            .send_json(
                Method::PATCH,
                &Self::object_path(account_id, level, external_id),
                &to_body(&params)?,
                Retry::RateLimit,
            )
            .await
    }

    /// `PUT /ads/accounts/:account_id/objects/:level/:external_id/status` -
    /// pause (`"paused"`) or switch on (`"active"`) one object. Switching on
    /// STARTS REAL AD SPEND: show the person the budget that pays (the object's
    /// own or its parent's) and get their yes first, then pass it as
    /// `confirm_budget_amount`. Switching on: needs a key with the opt-in
    /// `ads:spend` scope ("Start ads"); without it, and on Google
    /// (`api_draft_only`) always, the call answers `403 start_in_app` with the
    /// page where a person does it in the app. Pausing needs `None` and stays
    /// open (Google: `start_in_app`). Not retried after a 5xx or a connection
    /// error. Requires `ads:write`.
    pub async fn set_status(
        &self,
        account_id: &str,
        level: &str,
        external_id: &str,
        status: &str,
        confirm_budget_amount: Option<f64>,
    ) -> Result<Value, Error> {
        self.client
            .send_json(
                Method::PUT,
                &format!("{}/status", Self::object_path(account_id, level, external_id)),
                &status_body(status, confirm_budget_amount),
                Retry::RateLimit,
            )
            .await
    }

    /// `POST /ads/accounts/:account_id/objects/:level/:external_id/duplicate`:
    /// copy an object made outside OmniSocials; the copy is paused. Body:
    /// `deep` (default true), `rename`; `()` for none. Meta only today. It
    /// writes on the network: needs a key with the opt-in `ads:spend` scope
    /// ("Start ads"); without it, and on Google (`api_draft_only`) always, the
    /// call answers `403 start_in_app` with the page where a person does it in
    /// the app. Copy an OmniSocials campaign as a draft with
    /// [`crate::resources::Ads::duplicate`]. Requires `ads:write`.
    pub async fn duplicate<B: Serialize>(
        &self,
        account_id: &str,
        level: &str,
        external_id: &str,
        params: B,
    ) -> Result<Value, Error> {
        let path = format!("{}/duplicate", Self::object_path(account_id, level, external_id));
        post_optional(self.client, &path, &params, Retry::All).await
    }

    /// `GET /ads/accounts/:account_id/import` - the import state
    /// (`not_started`, `pending`, `done`, `failed`) and `campaigns_imported`.
    /// Requires `ads:read`.
    pub async fn import_status(&self, account_id: &str) -> Result<Value, Error> {
        self.client.get(&format!("{}/import", account_path(account_id)), Vec::new()).await
    }

    /// `POST /ads/accounts/:account_id/import` - read the ad account again at
    /// the next sync (within 15 minutes). The first import reads the last 90
    /// days; webhook `ads.initial_sync_completed` when it is done. Answers
    /// 202. Requires `ads:write`.
    pub async fn start_import(&self, account_id: &str) -> Result<Value, Error> {
        self.client.post_empty(&format!("{}/import", account_path(account_id))).await
    }

    /// `GET /ads/external-campaigns` - imported campaigns, by
    /// `ad_account_id`, `platform` or `status`; `limit` up to 200 and
    /// `offset`. Requires `ads:read`.
    pub async fn list_external<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/external-campaigns", &query)?, Vec::new()).await
    }

    /// `GET /ads/external-campaigns/:id` - one imported campaign with its
    /// daily numbers (`from`, `to`). Requires `ads:read`.
    pub async fn get_external<Q: Serialize>(&self, id: &str, query: Q) -> Result<Value, Error> {
        let path = format!("/ads/external-campaigns/{}", encode_path_segment(id));
        self.client.get(&with_query(&path, &query)?, Vec::new()).await
    }
}

impl AdTargeting<'_> {
    /// `GET /ads/targeting/search` - interests, places, languages, job titles
    /// and more. Query: `ad_account_id`, `type`, `q`, optional `limit` (up to
    /// 100) and `country`. Answers `items` with `id`, `name`, `type`, `path`
    /// and the audience size range.
    pub async fn search<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/targeting/search", &query)?, Vec::new()).await
    }

    /// `GET /ads/targeting/browse` - the categories of one `type` as a tree
    /// (`ad_account_id`, `type`).
    pub async fn browse<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/targeting/browse", &query)?, Vec::new()).await
    }

    /// `POST /ads/targeting/reach-estimate` - the audience size of a targeting
    /// (`ad_account_id`, `targeting`, optional `objective`). Answers `lower`,
    /// `upper`, `daily`.
    pub async fn reach_estimate<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/targeting/reach-estimate", &to_body(&params)?).await
    }

    /// `POST /ads/targeting/bid-suggestions` - a suggested bid and daily
    /// budget. Spends nothing.
    pub async fn bid_suggestions<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/targeting/bid-suggestions", &to_body(&params)?).await
    }
}

impl AdAudiences<'_> {
    fn audience_path(audience_id: &str) -> String {
        format!("/ads/audiences/{}", encode_path_segment(audience_id))
    }

    /// `GET /ads/audiences` - the audiences of `ad_account_id` (`cursor` for
    /// the next page). Requires `ads:read`.
    pub async fn list<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/audiences", &query)?, Vec::new()).await
    }

    /// `POST /ads/audiences` - create an audience: `ad_account_id`, `name`,
    /// optional `description`, `kind` (`customer_list`, `lookalike`,
    /// `company_list`), `lookalike` (`source_audience_id`, `country`, `ratio`
    /// 0.01 to 0.2), `customer_file_source` (Meta customer lists:
    /// `user_provided`, `partner_provided` or `both`, as the person says) and
    /// the first members as `schema` + `rows` (the shape of
    /// [`add_members`](AdAudiences::add_members); Pinterest makes a list only
    /// together with its first people). Requires `ads:write`.
    pub async fn create<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/audiences", &to_body(&params)?).await
    }

    /// `GET /ads/audiences/:audience_id?ad_account_id=` - one audience.
    /// Requires `ads:read`.
    pub async fn get(&self, audience_id: &str, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query(&Self::audience_path(audience_id), &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `PATCH /ads/audiences/:audience_id` - rename (`ad_account_id`, `name`,
    /// `description`). Requires `ads:write`.
    pub async fn update<B: Serialize>(&self, audience_id: &str, params: B) -> Result<Value, Error> {
        self.client.patch_json(&Self::audience_path(audience_id), &to_body(&params)?).await
    }

    /// `DELETE /ads/audiences/:audience_id?ad_account_id=` - delete an
    /// audience. Requires `ads:write`.
    pub async fn delete(&self, audience_id: &str, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query(&Self::audience_path(audience_id), &account_query(ad_account_id))?;
        self.client.delete(&path).await
    }

    /// `POST /ads/audiences/:audience_id/members` - add people to a customer
    /// list: `ad_account_id`, `schema` (`email`, `phone` with the country
    /// code, `first_name`, `last_name`, `city`, `state`, `zip`, `country`,
    /// `external_id`, `dob_year`, `gender`, `maid`) and `rows` (at most
    /// 10,000). Send the raw values: OmniSocials normalizes and SHA-256
    /// hashes them before they leave and stores nothing. Requires `ads:write`.
    pub async fn add_members<B: Serialize>(&self, audience_id: &str, params: B) -> Result<Value, Error> {
        let path = format!("{}/members", Self::audience_path(audience_id));
        self.client.post_json(&path, &to_body(&params)?).await
    }

    /// `DELETE /ads/audiences/:audience_id/members` - remove people (the same
    /// body as [`add_members`](AdAudiences::add_members)). Requires
    /// `ads:write`.
    pub async fn remove_members<B: Serialize>(&self, audience_id: &str, params: B) -> Result<Value, Error> {
        let path = format!("{}/members", Self::audience_path(audience_id));
        self.client.send_json(Method::DELETE, &path, &to_body(&params)?, Retry::All).await
    }

    /// `POST /ads/audiences/:audience_id/companies` - add companies to a
    /// company list (LinkedIn): `ad_account_id` and `companies` (each `name`,
    /// `domain` or `linkedin_url`). Requires `ads:write`.
    pub async fn add_companies<B: Serialize>(&self, audience_id: &str, params: B) -> Result<Value, Error> {
        let path = format!("{}/companies", Self::audience_path(audience_id));
        self.client.post_json(&path, &to_body(&params)?).await
    }
}

impl AdLeadForms<'_> {
    fn form_path(form_id: &str) -> String {
        format!("/ads/lead-forms/{}", encode_path_segment(form_id))
    }

    /// `GET /ads/lead-forms?ad_account_id=` - the lead forms. `id` is the
    /// OmniSocials form id, `external_id` the network's (use that as
    /// `settings.lead_form_id`). Requires `ads:read`.
    pub async fn list(&self, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query("/ads/lead-forms", &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `POST /ads/lead-forms` - create a lead form: `ad_account_id`, `name`,
    /// `privacy_policy_url`, `questions`, `thank_you`, optional `locale`,
    /// `headline`, `description` and `accept_terms` (the person accepts the
    /// network's lead ad terms; Pinterest refuses a form without it; only send
    /// true after the person said yes). Requires `ads:write`.
    pub async fn create<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/lead-forms", &to_body(&params)?).await
    }

    /// `GET /ads/lead-forms/:form_id` - one lead form. Requires `ads:read`.
    pub async fn get(&self, form_id: &str) -> Result<Value, Error> {
        self.client.get(&Self::form_path(form_id), Vec::new()).await
    }

    /// `PATCH /ads/lead-forms/:form_id` - `status` (`active` or `archived`)
    /// or `sync_enabled`. Requires `ads:write`.
    pub async fn update<B: Serialize>(&self, form_id: &str, params: B) -> Result<Value, Error> {
        self.client.patch_json(&Self::form_path(form_id), &to_body(&params)?).await
    }

    /// `DELETE /ads/lead-forms/:form_id` - archive a form; the leads already
    /// copied stay. Requires `ads:write`.
    pub async fn delete(&self, form_id: &str) -> Result<Value, Error> {
        self.client.delete(&Self::form_path(form_id)).await
    }

    /// `POST /ads/lead-forms/:form_id/test-leads` - send a test lead
    /// (optional `fields`; `()` for none), marked `is_test`. Requires
    /// `ads:write`.
    pub async fn send_test_lead<B: Serialize>(&self, form_id: &str, params: B) -> Result<Value, Error> {
        let path = format!("{}/test-leads", Self::form_path(form_id));
        post_optional(self.client, &path, &params, Retry::All).await
    }
}

impl AdLeads<'_> {
    /// `GET /ads/leads` - newest first, by `form_id`, `status` (`new`,
    /// `contacted`, `qualified`, `won`, `lost`, `spam`), `since`, `until`;
    /// `limit` up to 200 and `cursor`. Requires `ads:read`.
    pub async fn list<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/leads", &query)?, Vec::new()).await
    }

    /// `GET /ads/leads/counts` - `total` and `by_status`, optionally of one
    /// `form_id`. Requires `ads:read`.
    pub async fn counts<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/leads/counts", &query)?, Vec::new()).await
    }

    /// `PATCH /ads/leads/:lead_id` - set a lead's `status` or `note`.
    /// Requires `ads:write`.
    pub async fn update<B: Serialize>(&self, lead_id: &str, params: B) -> Result<Value, Error> {
        let path = format!("/ads/leads/{}", encode_path_segment(lead_id));
        self.client.patch_json(&path, &to_body(&params)?).await
    }
}

impl AdConversions<'_> {
    /// `GET /ads/conversions/destinations?ad_account_id=` - pixels, datasets,
    /// conversion tags and conversion actions. Requires `ads:read`.
    pub async fn destinations(&self, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query("/ads/conversions/destinations", &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `GET /ads/conversions/goals?ad_account_id=` - the events a campaign
    /// can optimise for. Requires `ads:read`.
    pub async fn goals(&self, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query("/ads/conversions/goals", &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `POST /ads/conversions/custom` - `ad_account_id`, `name`,
    /// `destination_id` and an `event` and/or `url_contains` rule. Requires
    /// `ads:write`.
    pub async fn create_custom<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/conversions/custom", &to_body(&params)?).await
    }

    /// `POST /ads/conversions/events` - `ad_account_id`, `destination_id` and
    /// up to 1,000 `events`. Send the raw email and phone: OmniSocials
    /// normalizes and SHA-256 hashes them before they leave and stores
    /// nothing. Requires `ads:write`.
    pub async fn send_events<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/conversions/events", &to_body(&params)?).await
    }
}

impl AdInsights<'_> {
    /// `POST /ads/insights` - live numbers (`ad_account_id`, `level`, `ids`,
    /// `metrics`, up to 2 `breakdowns`, `from`, `to`, `time_increment`).
    /// Answers `rows`.
    pub async fn query<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/insights", &to_body(&params)?).await
    }

    /// `GET /ads/campaigns/:id/analytics` - numbers of an OmniSocials
    /// campaign (`from`, `to`, `breakdown`: `ad` gives one row per ad,
    /// `time_increment`, `metrics`).
    pub async fn campaign<Q: Serialize>(&self, campaign_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&format!("{}/analytics", campaign_path(campaign_id)), &query)?;
        self.client.get(&path, Vec::new()).await
    }

    /// `GET /ads/campaigns/:id/ads/:ad_id/analytics` - numbers of one ad
    /// (`from`, `to`, `breakdown`).
    pub async fn ad<Q: Serialize>(&self, campaign_id: &str, ad_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&format!("{}/analytics", ad_path(campaign_id, ad_id)), &query)?;
        self.client.get(&path, Vec::new()).await
    }

    /// `GET /ads/timeline` - daily totals (`from`, `to`, `platform`,
    /// `source`: `all`, `omnisocials` or `external`).
    pub async fn timeline<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/timeline", &query)?, Vec::new()).await
    }

    /// `GET /ads/performance` - campaigns or ads ranked by one metric
    /// (`level`, `metric`, `order`, `limit`, `min_impressions`, `from`, `to`,
    /// `platform`).
    pub async fn performance<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/performance", &query)?, Vec::new()).await
    }

    /// `GET /ads/alerts` - rejected ads, held campaigns, sign-in or payment
    /// problems, no delivery.
    pub async fn alerts(&self) -> Result<Value, Error> {
        self.client.get("/ads/alerts", Vec::new()).await
    }
}

impl AdReports<'_> {
    /// `GET /ads/insights/reports` - the background reports of the workspace.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/ads/insights/reports", Vec::new()).await
    }

    /// `POST /ads/insights/reports` - start a long report (up to 730 days,
    /// the body of [`AdInsights::query`]). Answers 202.
    pub async fn start<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/insights/reports", &to_body(&params)?).await
    }

    /// `GET /ads/insights/reports/:report_id` - `running`, `done` (with
    /// `rows`) or `failed`.
    pub async fn get(&self, report_id: &str) -> Result<Value, Error> {
        let path = format!("/ads/insights/reports/{}", encode_path_segment(report_id));
        self.client.get(&path, Vec::new()).await
    }
}

impl AdCampaignAds<'_> {
    /// `GET /ads/campaigns/:id/ads` - the ads with their status, review and
    /// latest numbers. Requires `ads:read`.
    pub async fn list(&self, campaign_id: &str) -> Result<Value, Error> {
        self.client.get(&format!("{}/ads", campaign_path(campaign_id)), Vec::new()).await
    }

    /// `POST /ads/campaigns/:id/ads` - add up to 6 new ads to a campaign that
    /// runs, in the shape of a draft's ads (a `Vec<AdDraftAd>` or `json!`). The
    /// new ads are created PAUSED on the network; switch each one on with
    /// [`set_status`](AdCampaignAds::set_status). It changes a campaign on the
    /// network: needs a key with the opt-in `ads:spend` scope ("Start ads");
    /// without it, and on Google (`api_draft_only`) always, the call answers
    /// `403 start_in_app` with the page where a person does it in the app.
    /// Webhook `ad.created`. Requires `ads:write`.
    pub async fn add<B: Serialize>(&self, campaign_id: &str, ads: B) -> Result<Value, Error> {
        let body = json!({ "ads": to_body(&ads)? });
        self.client.post_json(&format!("{}/ads", campaign_path(campaign_id)), &body).await
    }

    /// `GET /ads/campaigns/:id/ads/:ad_id` - one ad. Requires `ads:read`.
    pub async fn get(&self, campaign_id: &str, ad_id: &str) -> Result<Value, Error> {
        self.client.get(&ad_path(campaign_id, ad_id), Vec::new()).await
    }

    /// `PUT /ads/campaigns/:id/ads/:ad_id/status` - pause (`"paused"`) or
    /// switch on (`"active"`) one ad. Switching on STARTS REAL AD SPEND: show
    /// the person the campaign budget and get their yes first, then pass it as
    /// `confirm_budget_amount`. Switching on: needs a key with the opt-in
    /// `ads:spend` scope ("Start ads"); without it, and on Google
    /// (`api_draft_only`) always, the call answers `403 start_in_app` with the
    /// page where a person does it in the app. Pausing needs `None` and stays
    /// open (Google: `start_in_app`). Webhook `ad.updated`. Not retried after a
    /// 5xx or a connection error: read the ad with [`get`](AdCampaignAds::get).
    /// Requires `ads:write`.
    pub async fn set_status(
        &self,
        campaign_id: &str,
        ad_id: &str,
        status: &str,
        confirm_budget_amount: Option<f64>,
    ) -> Result<Value, Error> {
        self.client
            .send_json(
                Method::PUT,
                &format!("{}/status", ad_path(campaign_id, ad_id)),
                &status_body(status, confirm_budget_amount),
                Retry::RateLimit,
            )
            .await
    }

    /// `GET /ads/campaigns/:id/ads/:ad_id/review` - `in_review`, `approved`
    /// or `rejected` with reasons. Requires `ads:read`.
    pub async fn review(&self, campaign_id: &str, ad_id: &str) -> Result<Value, Error> {
        self.client.get(&format!("{}/review", ad_path(campaign_id, ad_id)), Vec::new()).await
    }
}

impl AdPreviews<'_> {
    /// `POST /ads/previews` - previews of a new ad (`ad_account_id`, `ad`,
    /// optional `placements` and `objective`). Answers `previews`
    /// (`placement`, `html`, `url`).
    pub async fn create<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/previews", &to_body(&params)?).await
    }

    /// `GET /ads/campaigns/:id/ads/:ad_id/previews` - previews of an existing
    /// ad, optionally for some `placements` (empty = all).
    pub async fn for_ad<S: AsRef<str>>(&self, campaign_id: &str, ad_id: &str, placements: &[S]) -> Result<Value, Error> {
        let mut query = Vec::new();
        if !placements.is_empty() {
            let list: Vec<&str> = placements.iter().map(AsRef::as_ref).collect();
            query.push(("placements", list.join(",")));
        }
        self.client.get(&format!("{}/previews", ad_path(campaign_id, ad_id)), query).await
    }
}

impl AdRules<'_> {
    fn rule_path(rule_id: &str) -> String {
        format!("/ads/rules/{}", encode_path_segment(rule_id))
    }

    /// `GET /ads/rules`. Requires `ads:read`.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/ads/rules", Vec::new()).await
    }

    /// `POST /ads/rules` - create a rule (`name`, `conditions`, `action`,
    /// optional `enabled`, `scope`, `every_minutes` ...). A rule that can RAISE
    /// AD SPEND needs `confirm_max_daily_budget` = `action.max_daily_budget`,
    /// after the person agreed to it, and a key with `ads:spend` (else `403
    /// rule_raises_spend`). Preview it first with
    /// [`preview`](AdRules::preview). Not retried after a 5xx or a connection
    /// error. Requires `ads:write`.
    pub async fn create<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client
            .send_json(Method::POST, "/ads/rules", &to_body(&params)?, Retry::RateLimit)
            .await
    }

    /// `GET /ads/rules/templates` - ready-made rules. Requires `ads:read`.
    pub async fn templates(&self) -> Result<Value, Error> {
        self.client.get("/ads/rules/templates", Vec::new()).await
    }

    /// `POST /ads/rules/preview` - what an unsaved rule would match and do
    /// now. Changes nothing. Requires `ads:read`.
    pub async fn preview<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/rules/preview", &to_body(&params)?).await
    }

    /// `GET /ads/rules/activity` - every change the rules made (`rule_id`,
    /// `limit`). Requires `ads:read`.
    pub async fn activity<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/rules/activity", &query)?, Vec::new()).await
    }

    /// `POST /ads/rules/activity/:activity_id/undo` - undo a change a rule
    /// made. Undoing a pause or a budget cut STARTS OR RAISES REAL AD SPEND and
    /// needs `confirm_budget_amount` = the budget, after the person agreed to
    /// it, and a key with `ads:spend` (else, and on Google always, `403
    /// start_in_app`); undoing a budget raise needs `None`. Not retried after a
    /// 5xx or a connection error. Requires `ads:write`.
    pub async fn undo(&self, activity_id: &str, confirm_budget_amount: Option<f64>) -> Result<Value, Error> {
        let path = format!("/ads/rules/activity/{}/undo", encode_path_segment(activity_id));
        match confirm_budget_amount {
            Some(amount) => {
                self.client
                    .send_json(Method::POST, &path, &json!({ "confirm_budget_amount": amount }), Retry::RateLimit)
                    .await
            }
            None => self.client.post_empty_with(&path, Retry::RateLimit).await,
        }
    }

    /// `GET /ads/rules/:rule_id`. Requires `ads:read`.
    pub async fn get(&self, rule_id: &str) -> Result<Value, Error> {
        self.client.get(&Self::rule_path(rule_id), Vec::new()).await
    }

    /// `PATCH /ads/rules/:rule_id` - change a rule, also `enabled`. A changed
    /// action that can RAISE AD SPEND needs `confirm_max_daily_budget` again
    /// and a key with `ads:spend` (else `403 rule_raises_spend`). Not retried
    /// after a 5xx or a connection error. Requires `ads:write`.
    pub async fn update<B: Serialize>(&self, rule_id: &str, params: B) -> Result<Value, Error> {
        self.client
            .send_json(Method::PATCH, &Self::rule_path(rule_id), &to_body(&params)?, Retry::RateLimit)
            .await
    }

    /// `DELETE /ads/rules/:rule_id`. Requires `ads:write`.
    pub async fn delete(&self, rule_id: &str) -> Result<Value, Error> {
        self.client.delete(&Self::rule_path(rule_id)).await
    }

    /// `POST /ads/rules/:rule_id/preview` - what a saved rule would match and
    /// do now. Requires `ads:read`.
    pub async fn preview_saved(&self, rule_id: &str) -> Result<Value, Error> {
        self.client.post_empty(&format!("{}/preview", Self::rule_path(rule_id))).await
    }

    /// `GET /ads/rules/:rule_id/runs`. Requires `ads:read`.
    pub async fn runs<Q: Serialize>(&self, rule_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&format!("{}/runs", Self::rule_path(rule_id)), &query)?;
        self.client.get(&path, Vec::new()).await
    }
}

impl AdResults<'_> {
    /// `GET /ads/campaign-results` (`from`, `to`, `campaign_id`, `metric`).
    /// Requires `ads:read`.
    pub async fn list<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/campaign-results", &query)?, Vec::new()).await
    }

    /// `POST /ads/campaign-results` - upload `rows` (`day`, `metric`, `value`
    /// and the campaign; at most 5,000). Sending a day again replaces it.
    /// Requires `ads:write`.
    pub async fn upload<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/campaign-results", &to_body(&params)?).await
    }

    /// `GET /ads/campaign-results/totals` with the formulas. Requires
    /// `ads:read`.
    pub async fn totals<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/campaign-results/totals", &query)?, Vec::new()).await
    }

    /// `GET /ads/campaign-results/metrics` - your metric names and formulas.
    /// Requires `ads:read`.
    pub async fn metrics(&self) -> Result<Value, Error> {
        self.client.get("/ads/campaign-results/metrics", Vec::new()).await
    }

    /// `PUT /ads/campaign-results/metrics/:key` - `label`, `unit` (`number`,
    /// `money`, `percent`), `formula` (for example `results_revenue /
    /// spend`). Requires `ads:write`.
    pub async fn put_metric<B: Serialize>(&self, key: &str, params: B) -> Result<Value, Error> {
        let path = format!("/ads/campaign-results/metrics/{}", encode_path_segment(key));
        self.client.put_json(&path, &to_body(&params)?).await
    }
}

impl AdMedia<'_> {
    /// `GET /ads/media` (`ad_account_id`, `type`, `cursor`). Requires
    /// `ads:read`.
    pub async fn list<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query("/ads/media", &query)?, Vec::new()).await
    }

    /// `POST /ads/media` - `ad_account_id`, `type`, `url`, optional `name`.
    /// Requires `ads:write`.
    pub async fn upload<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/media", &to_body(&params)?).await
    }

    /// `GET /ads/media/:media_id` - the processing status (`ad_account_id`,
    /// `type`). Requires `ads:read`.
    pub async fn get<Q: Serialize>(&self, media_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&format!("/ads/media/{}", encode_path_segment(media_id)), &query)?;
        self.client.get(&path, Vec::new()).await
    }

    /// `DELETE /ads/media/:media_id` (`ad_account_id` and `type`). Requires
    /// `ads:write`.
    pub async fn delete<Q: Serialize>(&self, media_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&format!("/ads/media/{}", encode_path_segment(media_id)), &query)?;
        self.client.delete(&path).await
    }
}

impl AdPartnership<'_> {
    /// `GET /ads/partnership/content?ad_account_id=` - creator posts you may
    /// run as partnership ads. Requires `ads:read`.
    pub async fn list_content(&self, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query("/ads/partnership/content", &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `GET /ads/partnership/permissions?ad_account_id=`. Requires
    /// `ads:read`.
    pub async fn list_permissions(&self, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query("/ads/partnership/permissions", &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `POST /ads/partnership/permissions` - ask for or set a permission.
    /// Requires `ads:write`.
    pub async fn set_permission<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/partnership/permissions", &to_body(&params)?).await
    }
}

impl AdComments<'_> {
    fn comments_path(campaign_id: &str, ad_id: &str) -> String {
        format!("{}/comments", ad_path(campaign_id, ad_id))
    }

    /// `GET /ads/campaigns/:id/ads/:ad_id/comments` (`cursor`). Requires
    /// `ads:read`.
    pub async fn list<Q: Serialize>(&self, campaign_id: &str, ad_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&Self::comments_path(campaign_id, ad_id), &query)?;
        self.client.get(&path, Vec::new()).await
    }

    /// `POST .../comments/:comment_id/reply` - answer a comment. Requires
    /// `ads:write`.
    pub async fn reply(&self, campaign_id: &str, ad_id: &str, comment_id: &str, text: &str) -> Result<Value, Error> {
        let path = format!("{}/{}/reply", Self::comments_path(campaign_id, ad_id), encode_path_segment(comment_id));
        self.client.post_json(&path, &json!({ "text": text })).await
    }

    /// `POST .../comments/:comment_id/hide` - hide a comment (`hidden` true)
    /// or show it again (false). Requires `ads:write`.
    pub async fn hide(&self, campaign_id: &str, ad_id: &str, comment_id: &str, hidden: bool) -> Result<Value, Error> {
        let path = format!("{}/{}/hide", Self::comments_path(campaign_id, ad_id), encode_path_segment(comment_id));
        self.client.post_json(&path, &json!({ "hidden": hidden })).await
    }

    /// `DELETE .../comments/:comment_id`. Requires `ads:write`.
    pub async fn delete(&self, campaign_id: &str, ad_id: &str, comment_id: &str) -> Result<Value, Error> {
        let path = format!("{}/{}", Self::comments_path(campaign_id, ad_id), encode_path_segment(comment_id));
        self.client.delete(&path).await
    }
}

impl AdAccounts<'_> {
    async fn read(&self, account_id: &str, part: &str) -> Result<Value, Error> {
        self.client.get(&format!("{}/{part}", account_path(account_id)), Vec::new()).await
    }

    /// `GET /ads/accounts/:account_id/health` - status, payment, spend limit,
    /// Page, pixel; failed checks carry a `fix_url`. Requires `ads:read`.
    pub async fn health(&self, account_id: &str) -> Result<Value, Error> {
        self.read(account_id, "health").await
    }

    /// `GET /ads/accounts/:account_id/finance` - spend limit, amount spent,
    /// balance and payment method. Requires `ads:read`.
    pub async fn finance(&self, account_id: &str) -> Result<Value, Error> {
        self.read(account_id, "finance").await
    }

    /// `PATCH /ads/accounts/:account_id/settings` - `spend_cap`,
    /// `dsa_beneficiary`, `dsa_payor`, `name`. A HIGHER SPEND CAP lets the
    /// account spend more: show the person the new cap and get their yes first,
    /// then pass it as `confirm_budget_amount`; removing the cap (`"spend_cap":
    /// null`) needs `"confirm_budget_amount": "remove"`. A spend cap change:
    /// needs a key with the opt-in `ads:spend` scope ("Start ads"); without it,
    /// and on Google (`api_draft_only`) always, the call answers `403
    /// start_in_app` with the page where a person does it in the app. The name
    /// and the DSA names stay open. Not retried after a 5xx or a connection
    /// error: read [`finance`](AdAccounts::finance). Requires `ads:write`.
    pub async fn update_settings<B: Serialize>(&self, account_id: &str, params: B) -> Result<Value, Error> {
        self.client
            .send_json(
                Method::PATCH,
                &format!("{}/settings", account_path(account_id)),
                &to_body(&params)?,
                Retry::RateLimit,
            )
            .await
    }

    /// `GET /ads/accounts/:account_id/users` - people with access. Requires
    /// `ads:read`.
    pub async fn list_users(&self, account_id: &str) -> Result<Value, Error> {
        self.read(account_id, "users").await
    }

    /// `POST /ads/accounts/:account_id/users` - give access (LinkedIn:
    /// `role` and `email` or `member_id`). Requires `ads:write`.
    pub async fn add_user<B: Serialize>(&self, account_id: &str, params: B) -> Result<Value, Error> {
        self.client
            .post_json(&format!("{}/users", account_path(account_id)), &to_body(&params)?)
            .await
    }

    /// `DELETE /ads/accounts/:account_id/users/:user_id`. Requires
    /// `ads:write`.
    pub async fn remove_user(&self, account_id: &str, user_id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("{}/users/{}", account_path(account_id), encode_path_segment(user_id)))
            .await
    }

    /// `GET /ads/accounts/:account_id/activity` - the change log (`since`,
    /// `until`, `cursor`). Requires `ads:read`.
    pub async fn activity<Q: Serialize>(&self, account_id: &str, query: Q) -> Result<Value, Error> {
        let path = with_query(&format!("{}/activity", account_path(account_id)), &query)?;
        self.client.get(&path, Vec::new()).await
    }

    /// `GET /ads/accounts/:account_id/businesses` - business portfolios
    /// (Meta), Business Centers (TikTok). Requires `ads:read`.
    pub async fn businesses(&self, account_id: &str) -> Result<Value, Error> {
        self.read(account_id, "businesses").await
    }

    /// `GET /ads/accounts/:account_id/dsa` - the EU DSA payer and beneficiary
    /// (Meta). Requires `ads:read`.
    pub async fn dsa(&self, account_id: &str) -> Result<Value, Error> {
        self.read(account_id, "dsa").await
    }

    /// `GET /ads/accounts/:account_id/hierarchy` - the manager account
    /// hierarchy (Google). Requires `ads:read`.
    pub async fn hierarchy(&self, account_id: &str) -> Result<Value, Error> {
        self.read(account_id, "hierarchy").await
    }

    /// `GET /ads/accounts/:account_id/identities` - Pages, profiles and
    /// identities an ad can run as. Requires `ads:read`.
    pub async fn identities(&self, account_id: &str) -> Result<Value, Error> {
        self.read(account_id, "identities").await
    }
}

impl AdReachFrequency<'_> {
    fn prediction_path(prediction_id: &str) -> String {
        format!("/ads/reach-frequency/{}", encode_path_segment(prediction_id))
    }

    /// `POST /ads/reach-frequency` - create a prediction: `ad_account_id`,
    /// `objective` (`awareness`, `engagement` or `video_views`), `targeting`
    /// (exactly one country in `countries`, optional `age_min`, `age_max`,
    /// `genders`), `start_at`, `end_at`, and `budget` (major units) or
    /// `reach` (people to reach), optional `frequency_cap` and
    /// `frequency_interval_days`. Meta needs the reach and frequency
    /// capability on the ad account. Spends nothing. Requires `ads:write`.
    pub async fn create<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/reach-frequency", &to_body(&params)?).await
    }

    /// `GET /ads/reach-frequency/:prediction_id?ad_account_id=`. Requires
    /// `ads:read`.
    pub async fn get(&self, prediction_id: &str, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query(&Self::prediction_path(prediction_id), &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `DELETE /ads/reach-frequency/:prediction_id?ad_account_id=` - cancel a
    /// prediction or a reservation. Requires `ads:write`.
    pub async fn cancel(&self, prediction_id: &str, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query(&Self::prediction_path(prediction_id), &account_query(ad_account_id))?;
        self.client.delete(&path).await
    }

    /// `POST /ads/reach-frequency/:prediction_id/reserve` - COMMITS REAL AD
    /// SPEND. Show the person the prediction's budget and get their yes first;
    /// `confirm_budget_amount` must equal it. Needs a key with the opt-in
    /// `ads:spend` scope ("Start ads"); without it, and on Google
    /// (`api_draft_only`) always, the call answers `403 start_in_app` with the
    /// page where a person does it in the app. Not retried after a 5xx or a
    /// connection error: read the prediction with
    /// [`get`](AdReachFrequency::get). Requires `ads:write`.
    pub async fn reserve(&self, prediction_id: &str, ad_account_id: &str, confirm_budget_amount: f64) -> Result<Value, Error> {
        let body = json!({ "ad_account_id": ad_account_id, "confirm_budget_amount": confirm_budget_amount });
        self.client
            .send_json(
                Method::POST,
                &format!("{}/reserve", Self::prediction_path(prediction_id)),
                &body,
                Retry::RateLimit,
            )
            .await
    }
}

impl AdStudies<'_> {
    /// `GET /ads/studies?ad_account_id=`. Requires `ads:read`.
    pub async fn list(&self, ad_account_id: &str) -> Result<Value, Error> {
        let path = with_query("/ads/studies", &account_query(ad_account_id))?;
        self.client.get(&path, Vec::new()).await
    }

    /// `POST /ads/studies` - an A/B test between existing campaigns or ad sets.
    /// Needs a key with the opt-in `ads:spend` scope ("Start ads"); without it,
    /// and on Google (`api_draft_only`) always, the call answers `403
    /// start_in_app` with the page where a person does it in the app. Requires
    /// `ads:write`.
    pub async fn create<B: Serialize>(&self, params: B) -> Result<Value, Error> {
        self.client.post_json("/ads/studies", &to_body(&params)?).await
    }
}

impl AdBudgetSchedules<'_> {
    fn schedules_path(campaign_id: &str) -> String {
        format!("{}/budget-schedules", campaign_path(campaign_id))
    }

    /// `GET /ads/campaigns/:id/budget-schedules`. Requires `ads:read`.
    pub async fn list(&self, campaign_id: &str) -> Result<Value, Error> {
        self.client.get(&Self::schedules_path(campaign_id), Vec::new()).await
    }

    /// `POST /ads/campaigns/:id/budget-schedules` - `start_at`, `end_at`,
    /// `budget_increase` (`type` `absolute` = the extra per day, or
    /// `multiplier` = a whole percentage on top, 50 = 50% more; `value`) and
    /// `confirm_budget_amount`. It is NEW AD SPEND in that period: show the
    /// person the amount and get their yes first; `confirm_budget_amount` = the
    /// daily total during the period (the daily budget plus the increase),
    /// under the OmniSocials daily cap. Needs a key with the opt-in `ads:spend`
    /// scope ("Start ads"); without it, and on Google (`api_draft_only`)
    /// always, the call answers `403 start_in_app` with the page where a person
    /// does it in the app. Not retried after a 5xx or a connection error.
    /// Requires `ads:write`.
    pub async fn create<B: Serialize>(&self, campaign_id: &str, params: B) -> Result<Value, Error> {
        self.client
            .send_json(Method::POST, &Self::schedules_path(campaign_id), &to_body(&params)?, Retry::RateLimit)
            .await
    }

    /// `DELETE /ads/campaigns/:id/budget-schedules/:schedule_id`. Requires
    /// `ads:write`.
    pub async fn delete(&self, campaign_id: &str, schedule_id: &str) -> Result<Value, Error> {
        let path = format!("{}/{}", Self::schedules_path(campaign_id), encode_path_segment(schedule_id));
        self.client.delete(&path).await
    }
}

impl AdGoogle<'_> {
    async fn read<Q: Serialize>(&self, report: &str, query: Q) -> Result<Value, Error> {
        self.client.get(&with_query(&format!("/ads/google/{report}"), &query)?, Vec::new()).await
    }

    /// `GET /ads/google/search-terms` (`campaign_id` or
    /// `external_campaign_id`, `from`, `to`, `limit`).
    pub async fn search_terms<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("search-terms", query).await
    }

    /// `GET /ads/google/recommendations` (`types`).
    pub async fn recommendations<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("recommendations", query).await
    }

    /// `GET /ads/google/keywords` (`external_campaign_id`,
    /// `external_ad_set_id`).
    pub async fn keywords<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("keywords", query).await
    }

    /// `GET /ads/google/assets` - sitelinks, callouts and structured snippets
    /// (`type`).
    pub async fn assets<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("assets", query).await
    }

    /// `GET /ads/google/shared-budgets`.
    pub async fn shared_budgets<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("shared-budgets", query).await
    }

    /// `GET /ads/google/bid-strategies` - portfolio bid strategies.
    pub async fn bid_strategies<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("bid-strategies", query).await
    }

    /// `GET /ads/google/ad-schedule` - days and hours of a campaign
    /// (`external_campaign_id`).
    pub async fn ad_schedule<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("ad-schedule", query).await
    }

    /// `GET /ads/google/labels`.
    pub async fn labels<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("labels", query).await
    }

    /// `GET /ads/google/local-services-leads` (`since`).
    pub async fn local_services_leads<Q: Serialize>(&self, query: Q) -> Result<Value, Error> {
        self.read("local-services-leads", query).await
    }
}

/// A POST body that may be empty, for the campaign-level calls in `ads.rs`.
pub(crate) async fn post_maybe_empty<B: Serialize>(client: &Client, path: &str, body: &B) -> Result<Value, Error> {
    post_optional(client, path, body, Retry::All).await
}

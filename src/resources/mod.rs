//! Resource accessors, one per API endpoint group. Obtain them from
//! [`crate::Client`]: `client.posts()`, `client.media()`, and so on.

mod accounts;
mod ads;
mod ads_parity;
mod analytics;
mod approval_workflows;
mod audio;
mod automations;
mod feature_requests;
mod folders;
mod hashtag_sets;
mod inbox;
mod limits;
mod locations;
mod media;
mod pinterest;
mod posts;
mod tags;
mod webhooks;

pub use accounts::Accounts;
pub use ads::{AdDrafts, Ads};
pub use ads_parity::{
    AdAccounts, AdAudiences, AdBudgetSchedules, AdCampaignAds, AdComments, AdConversions, AdGoogle,
    AdInsights, AdLeadForms, AdLeads, AdMedia, AdObjects, AdPartnership, AdPreviews,
    AdReachFrequency, AdReports, AdResults, AdRules, AdStudies, AdTargeting,
};
pub use analytics::Analytics;
pub use approval_workflows::ApprovalWorkflows;
pub use audio::Audio;
pub use automations::Automations;
pub use feature_requests::FeatureRequests;
pub use folders::Folders;
pub use hashtag_sets::HashtagSets;
pub use inbox::Inbox;
pub use limits::Limits;
pub use locations::Locations;
pub use media::Media;
pub use pinterest::Pinterest;
pub use posts::Posts;
pub use tags::Tags;
pub use webhooks::Webhooks;

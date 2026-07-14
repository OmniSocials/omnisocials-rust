//! Resource accessors, one per API endpoint group. Obtain them from
//! [`crate::Client`]: `client.posts()`, `client.media()`, and so on.

mod accounts;
mod analytics;
mod folders;
mod locations;
mod media;
mod posts;
mod webhooks;

pub use accounts::Accounts;
pub use analytics::Analytics;
pub use folders::Folders;
pub use locations::Locations;
pub use media::Media;
pub use posts::Posts;
pub use webhooks::Webhooks;

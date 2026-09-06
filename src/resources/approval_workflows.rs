use serde_json::Value;

use crate::client::Client;
use crate::error::Error;

/// `client.approval_workflows()`: the approval workflows configured in the
/// OmniSocials dashboard (Approvals). Route a post through one at create
/// time via [`crate::CreatePostParams::approval_workflow_id`].
#[derive(Debug, Clone, Copy)]
pub struct ApprovalWorkflows<'a> {
    pub(crate) client: &'a Client,
}

impl ApprovalWorkflows<'_> {
    /// `GET /approval-workflows` - the workflows this workspace can use
    /// (company-wide plus workspace-bound), with steps and named approvers.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/approval-workflows", Vec::new()).await
    }
}

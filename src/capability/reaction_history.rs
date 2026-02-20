use crate::domain::person_uuid::PersonUuid;
use chrono::{DateTime, Utc};

pub trait ReactionHistoryCapability {
    async fn record_reaction(
        &self,
        person_uuid: &PersonUuid,
        action_kind: &str,
    ) -> Result<(), String>;

    async fn has_reacted_since(
        &self,
        person_uuid: &PersonUuid,
        since: DateTime<Utc>,
    ) -> Result<bool, String>;
}

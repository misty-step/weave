use serde::Serialize;

/// Historical movement shape retained for saved evidence packs and reports.
/// Live collection from the retired ledger is no longer supported.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CardMovement {
    pub card_id: String,
    pub repo: String,
    pub event_type: String,
    pub actor: String,
    pub at: String,
    pub summary: String,
    pub source: String,
}

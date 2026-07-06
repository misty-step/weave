pub mod bb;
pub mod feed;
pub mod git;
pub mod moments;
pub mod powder;
pub mod receipts;

/// One line of "how this retro knows what it knows" -- rendered verbatim
/// into the report's provenance disclosure. Every collector produces zero
/// or more of these regardless of whether it found activity, so a quiet
/// source (no bb runs, Powder unconfigured) is visible as a stated gap
/// rather than an unexplained absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNote {
    pub source: String,
    pub note: String,
}

impl SourceNote {
    pub fn new(source: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            note: note.into(),
        }
    }
}

use anyhow::{Context, Result};

use crate::citation_gate;
use crate::pack::EvidencePack;
use crate::spec::{Narrative, NarrativeStatus};

/// Prompt-shape version, recorded in the report footer alongside the pack
/// schema version and judge model (oracle findings ruled binding
/// 2026-07-06, SRE-postmortem convention: snapshot every input version so a
/// bad report is diagnosable later). Bump whenever `build_prompt`'s
/// instructions or citation-format contract changes.
pub const PROMPT_VERSION: &str = "weave-fleet-retro-narrative-v1";

/// Cheap-default/escalate-on-failure model routing (doctrine: cheap tier
/// default, escalate on gate failure; roster's `primitives/tiers.yaml`
/// openrouter-class/codex-class `bb` mappings, prefix stripped since that's
/// a roster dispatch-tier symbol, not the literal OpenRouter model id).
/// Bounded retries per the oracle's cited 2026 production consensus
/// (bounded max_swaps, differentiate failure type): try the cheap model
/// twice (the second attempt absorbs a transient parse/formatting miss, not
/// a capability gap) before escalating once to a stronger model, then fail
/// open. Three attempts total, never more.
const ATTEMPT_MODELS: [&str; 3] = [
    "deepseek/deepseek-v4-flash",
    "deepseek/deepseek-v4-flash",
    "moonshotai/kimi-k2.7-code",
];

/// The external boundary this module talks to. A trait, not a concrete
/// `OpenRouterClient` type, so the retry/escalate/fail-open state machine in
/// `synthesize` is unit-testable against a scripted fake -- per house style,
/// only an external boundary (the OpenRouter API) is an acceptable mocking
/// point; nothing internal to this crate is mocked.
pub trait SynthesisClient {
    fn complete(&self, model: &str, prompt: &str) -> Result<String>;
}

/// Real OpenRouter chat-completions client. `OPENROUTER_API_KEY` is read via
/// env (falling back to `~/.secrets`, matching every other credential this
/// crate reads) -- never logged, never embedded in generated output.
pub struct OpenRouterClient {
    api_key: String,
}

impl OpenRouterClient {
    /// `None` when unconfigured, not an error -- a retro run without an
    /// OpenRouter key is a fail-open case (deterministic tables-only report
    /// with a visible banner), not a hard failure of the whole generator.
    pub fn from_env() -> Option<Self> {
        let api_key = crate::secrets::env_or_secrets_file("OPENROUTER_API_KEY")?;
        if api_key.trim().is_empty() {
            return None;
        }
        Some(Self { api_key })
    }
}

impl SynthesisClient for OpenRouterClient {
    fn complete(&self, model: &str, prompt: &str) -> Result<String> {
        let body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.2,
        });
        let response = ureq::post("https://openrouter.ai/api/v1/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(body)
            .context("OpenRouter chat-completions request failed")?;
        let value: serde_json::Value = response
            .into_json()
            .context("OpenRouter response was not valid JSON")?;
        value["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .context("OpenRouter response missing choices[0].message.content")
    }
}

/// Build the synthesis prompt from an `EvidencePack`: every item serialized
/// compactly (id, ts, source, kind, title, refs, excerpt) as the ONLY
/// grounding context, with an explicit instruction that every factual claim
/// must carry an inline `[id]` citation to one of the listed items and that
/// items whose `source` starts with `moment:` are curated anomaly signals
/// (Bitterblossom's flight-recorder scorer) that should be foregrounded,
/// not buried under routine commit/card traffic.
fn build_prompt(pack: &EvidencePack) -> String {
    let mut evidence = String::new();
    for item in &pack.items {
        evidence.push_str(&format!(
            "- id={} ts={} source={} kind={} title=\"{}\" refs={:?} excerpt=\"{}\"\n",
            item.id, item.ts, item.source, item.kind, item.title, item.refs, item.excerpt
        ));
    }
    format!(
        "You are writing the narrative section of a fleet-activity retro. Below is \
the complete, closed set of evidence for this report's window -- an \
EvidencePack. Every factual claim you make MUST end with an inline citation \
in the exact form [id], where id is copied verbatim from one of the items \
below. Never cite an id that is not listed. Never make a claim with no \
citation. Items whose source starts with \"moment:\" are curated, \
already-scored anomaly signals (failures, recoveries, cost anomalies, \
surprises) from Bitterblossom's flight-recorder scorer -- foreground these \
as the most newsworthy part of the narrative when present, don't bury them \
under routine commit traffic.\n\n\
Write 2-5 short paragraphs: what mattered, significance-ranked; call out \
anomalies explicitly; note causal threads across sources when evident; \
suggest 0-3 concrete follow-up candidates if the evidence supports them. \
Plain prose, no markdown headings, no bullet lists -- paragraphs only. Every \
non-blank line must end with at least one [id] citation.\n\n\
EVIDENCE:\n{evidence}"
    )
}

/// Outcome of the whole cheap-default/escalate/retry/fail-open cascade:
/// either a gate-passed narrative or an explicit fail-open, plus the footer
/// metadata (judge model, gate status) every run records regardless of
/// which path it took.
pub struct SynthesisOutcome {
    pub narrative: Narrative,
    pub judge: String,
    pub gate_status: String,
}

/// Run the synthesis cascade against `pack`: cheap model, cheap model
/// retry, escalated model, in that fixed bounded order, gating each
/// attempt's output through `citation_gate::validate_citations` before
/// accepting it. Falls open to a deterministic banner (no narrative prose)
/// when every attempt either fails to reach the model or fails the gate --
/// this is the "model unreachable -> deterministic report + banner" path
/// the card requires, proven by this function's own unit tests with a
/// client double that always errors, with no network involved.
pub fn synthesize(client: &dyn SynthesisClient, pack: &EvidencePack) -> SynthesisOutcome {
    let prompt = build_prompt(pack);

    for (attempt, model) in ATTEMPT_MODELS.iter().enumerate() {
        let text = match client.complete(model, &prompt) {
            Ok(text) => text,
            Err(err) => {
                eprintln!(
                    "fleet-retro: synthesis attempt {} ({model}) failed to reach the model: {err}",
                    attempt + 1
                );
                continue;
            }
        };
        match citation_gate::validate_citations(&text, pack) {
            Ok(citations) => {
                let blocks: Vec<String> = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect();
                return SynthesisOutcome {
                    narrative: Narrative {
                        status: NarrativeStatus::Ok { blocks, citations },
                    },
                    judge: model.to_string(),
                    gate_status: format!(
                        "passed on attempt {} of {} ({model})",
                        attempt + 1,
                        ATTEMPT_MODELS.len()
                    ),
                };
            }
            Err(rejection) => {
                eprintln!(
                    "fleet-retro: synthesis attempt {} ({model}) rejected by the citation gate: {}",
                    attempt + 1,
                    rejection.reason
                );
                continue;
            }
        }
    }

    SynthesisOutcome {
        narrative: Narrative {
            status: NarrativeStatus::FailedOpen {
                reason: format!(
                    "exhausted {} attempts ({} cheap, {} escalated) without a gate-passing narrative; showing the deterministic tables-only report",
                    ATTEMPT_MODELS.len(),
                    ATTEMPT_MODELS.len() - 1,
                    1
                ),
            },
        },
        judge: "none".to_string(),
        gate_status: "fail-open: no attempt passed the citation gate".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{EvidenceItem, EvidencePack, PackWindow};
    use std::cell::RefCell;

    fn pack_with_one_item() -> EvidencePack {
        EvidencePack {
            schema_version: "weave.evidence-pack.v1".to_string(),
            window: PackWindow {
                since: "2026-07-04T21:00:00Z".to_string(),
                until: "2026-07-05T21:00:00Z".to_string(),
            },
            items: vec![EvidenceItem {
                id: "aaaaaaaaaaaaaaaa".to_string(),
                ts: "2026-07-05T04:00:00Z".to_string(),
                source: "git:/dev/landmark".to_string(),
                kind: "commit".to_string(),
                title: "landmark shipped a fix".to_string(),
                refs: vec![],
                excerpt: String::new(),
            }],
        }
    }

    /// A scripted double standing in for the OpenRouter boundary: each call
    /// pops the next canned response off a queue. This is the sanctioned
    /// mocking point (an external boundary), not an internal collaborator.
    struct ScriptedClient {
        responses: RefCell<Vec<Result<String, String>>>,
        calls: RefCell<Vec<String>>,
    }

    impl ScriptedClient {
        fn new(responses: Vec<Result<String, String>>) -> Self {
            Self {
                responses: RefCell::new(responses),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl SynthesisClient for ScriptedClient {
        fn complete(&self, model: &str, _prompt: &str) -> Result<String> {
            self.calls.borrow_mut().push(model.to_string());
            match self.responses.borrow_mut().remove(0) {
                Ok(text) => Ok(text),
                Err(err) => Err(anyhow::anyhow!(err)),
            }
        }
    }

    #[test]
    fn cheap_model_success_on_first_attempt_never_escalates() {
        let pack = pack_with_one_item();
        let client = ScriptedClient::new(vec![Ok(
            "Landmark shipped a fix today [aaaaaaaaaaaaaaaa].".to_string(),
        )]);

        let outcome = synthesize(&client, &pack);

        assert_eq!(outcome.judge, "deepseek/deepseek-v4-flash");
        assert!(outcome.gate_status.contains("attempt 1"));
        let NarrativeStatus::Ok { blocks, citations } = outcome.narrative.status else {
            panic!("expected an accepted narrative");
        };
        assert_eq!(blocks.len(), 1);
        assert_eq!(citations.len(), 1);
        assert_eq!(client.calls.borrow().len(), 1);
    }

    #[test]
    fn model_unreachable_every_attempt_fails_open_with_a_visible_reason() {
        // The exact acceptance criterion: "Fail-open path proven (model
        // unreachable -> deterministic report + banner)".
        let pack = pack_with_one_item();
        let client = ScriptedClient::new(vec![
            Err("connection refused".to_string()),
            Err("connection refused".to_string()),
            Err("connection refused".to_string()),
        ]);

        let outcome = synthesize(&client, &pack);

        assert_eq!(outcome.judge, "none");
        assert!(outcome.gate_status.starts_with("fail-open"));
        let NarrativeStatus::FailedOpen { reason } = outcome.narrative.status else {
            panic!("expected a fail-open narrative when the model is unreachable");
        };
        assert!(reason.contains("exhausted"));
        assert_eq!(client.calls.borrow().len(), 3);
    }

    #[test]
    fn fabricated_citation_every_attempt_escalates_then_fails_open() {
        let pack = pack_with_one_item();
        let client = ScriptedClient::new(vec![
            Ok("Something happened [ffffffffffffffff].".to_string()),
            Ok("Something happened [ffffffffffffffff].".to_string()),
            Ok("Something happened [ffffffffffffffff].".to_string()),
        ]);

        let outcome = synthesize(&client, &pack);

        assert_eq!(outcome.judge, "none");
        let NarrativeStatus::FailedOpen { .. } = outcome.narrative.status else {
            panic!("expected fail-open when every attempt cites a fabricated id");
        };
        // Escalation order: cheap, cheap, escalated -- proving the routing
        // doctrine ("cheap-default/escalate-on-failure routing").
        assert_eq!(
            *client.calls.borrow(),
            vec![
                "deepseek/deepseek-v4-flash".to_string(),
                "deepseek/deepseek-v4-flash".to_string(),
                "moonshotai/kimi-k2.7-code".to_string(),
            ]
        );
    }

    #[test]
    fn escalated_model_recovers_after_two_cheap_failures() {
        let pack = pack_with_one_item();
        let client = ScriptedClient::new(vec![
            Ok("Fabricated claim [ffffffffffffffff].".to_string()),
            Ok("Fabricated claim [ffffffffffffffff].".to_string()),
            Ok("Landmark shipped a fix today [aaaaaaaaaaaaaaaa].".to_string()),
        ]);

        let outcome = synthesize(&client, &pack);

        assert_eq!(outcome.judge, "moonshotai/kimi-k2.7-code");
        assert!(outcome.gate_status.contains("attempt 3"));
        assert!(matches!(
            outcome.narrative.status,
            NarrativeStatus::Ok { .. }
        ));
    }
}

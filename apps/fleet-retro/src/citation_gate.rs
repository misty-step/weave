use crate::pack::EvidencePack;
use crate::spec::Citation;

/// Every pack item id is a 16-hex-digit `stable_id` (`pack.rs`). Anchoring
/// the citation token to that exact shape means a narrative can never
/// accidentally "cite" ordinary bracketed markdown (`[a link]`, `[1]`
/// footnote style) -- only something that looks like a real pack-item id is
/// ever treated as a citation claim in the first place.
fn citation_regex() -> regex::Regex {
    regex::Regex::new(r"\[([0-9a-f]{16})\]").expect("static citation-token pattern is valid")
}

/// Why the gate rejected a synthesis attempt. Deliberately just a string:
/// this is operator/log-facing diagnostic text, not a typed error a caller
/// branches on -- every caller either accepts the pack of citations or
/// retries/escalates/fails open, and the reason exists to make *that*
/// decision auditable in the report footer and stderr, not to drive further
/// control flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRejection {
    pub reason: String,
}

/// The deterministic citation gate (weave-923, oracle findings ruled
/// binding 2026-07-06): tier 1 of a cheapest-first cascade only -- an
/// existence check, not claim-level entailment. Two checks, both purely
/// mechanical over the narrative text and the pack, no model judgment:
///
/// 1. **Fabrication**: every `[id]` token in the narrative must name a pack
///    item that actually exists. This alone catches "up to 57%" of
///    hallucinated citations per the oracle's cited CiteCheck research
///    (arXiv:2605.27700) -- ship this tier first, don't gate v1 on the
///    heavier claim-level entailment tier being built.
/// 2. **Uncited claims**: every non-empty, non-heading line of narrative
///    prose must carry at least one citation token. This is still a
///    mechanical structural check (does the line contain a bracket token at
///    all), not semantic entailment -- entailment (does the cited item
///    actually support *this specific* sentence) is an explicitly deferred
///    later child, per Governance Decay (arXiv:2606.22528): the gate stays
///    external and deterministic forever, never folded into the synthesis
///    model's own prompt, and never grows model judgment of its own.
///
/// Returns the deduplicated, pack-resolved `Citation` list on success (for
/// `render.rs`'s hover/tap links) or the first rejection reason found.
pub fn validate_citations(
    narrative: &str,
    pack: &EvidencePack,
) -> Result<Vec<Citation>, GateRejection> {
    let re = citation_regex();
    let mut seen_ids: Vec<String> = Vec::new();
    let mut citations: Vec<Citation> = Vec::new();

    for line in narrative.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut line_has_citation = false;
        for capture in re.captures_iter(trimmed) {
            line_has_citation = true;
            let id = capture[1].to_string();
            let Some(item) = pack.items.iter().find(|item| item.id == id) else {
                return Err(GateRejection {
                    reason: format!(
                        "fabricated citation: [{id}] does not exist in the evidence pack"
                    ),
                });
            };
            if !seen_ids.contains(&id) {
                seen_ids.push(id.clone());
                citations.push(Citation {
                    id,
                    title: item.title.clone(),
                });
            }
        }
        if !line_has_citation {
            return Err(GateRejection {
                reason: format!(
                    "uncited claim: line has no [id] citation: \"{}\"",
                    if trimmed.len() > 80 {
                        format!("{}…", &trimmed[..80])
                    } else {
                        trimmed.to_string()
                    }
                ),
            });
        }
    }

    Ok(citations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{EvidenceItem, PackWindow};

    fn pack_with(items: Vec<EvidenceItem>) -> EvidencePack {
        EvidencePack {
            schema_version: "weave.evidence-pack.v1".to_string(),
            window: PackWindow {
                since: "2026-07-04T21:00:00Z".to_string(),
                until: "2026-07-05T21:00:00Z".to_string(),
            },
            items,
        }
    }

    fn item(id: &str, title: &str) -> EvidenceItem {
        EvidenceItem {
            id: id.to_string(),
            ts: "2026-07-05T04:00:00Z".to_string(),
            source: "git:/dev/landmark".to_string(),
            kind: "commit".to_string(),
            title: title.to_string(),
            refs: vec![],
            excerpt: String::new(),
        }
    }

    #[test]
    fn accepts_a_narrative_where_every_line_cites_a_real_item() {
        let pack = pack_with(vec![item("aaaaaaaaaaaaaaaa", "landmark shipped a fix")]);
        let narrative = "# What mattered\n\nLandmark shipped a fix today [aaaaaaaaaaaaaaaa].\n";

        let result = validate_citations(narrative, &pack);

        let citations = result.expect("well-formed narrative should pass the gate");
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "aaaaaaaaaaaaaaaa");
        assert_eq!(citations[0].title, "landmark shipped a fix");
    }

    #[test]
    fn rejects_a_citation_to_an_id_that_does_not_exist_in_the_pack() {
        // The exact acceptance criterion this card names: "fabricated-
        // citation test case rejected in CI".
        let pack = pack_with(vec![item("aaaaaaaaaaaaaaaa", "real item")]);
        let narrative = "The fleet shipped something big [ffffffffffffffff].\n";

        let result = validate_citations(narrative, &pack);

        let rejection = result.expect_err("a citation to a nonexistent id must be rejected");
        assert!(rejection.reason.contains("fabricated citation"));
        assert!(rejection.reason.contains("ffffffffffffffff"));
    }

    #[test]
    fn rejects_a_factual_line_with_no_citation_at_all() {
        let pack = pack_with(vec![item("aaaaaaaaaaaaaaaa", "real item")]);
        let narrative = "Landmark shipped a fix today with no citation at all.\n";

        let result = validate_citations(narrative, &pack);

        let rejection = result.expect_err("an uncited claim line must be rejected");
        assert!(rejection.reason.contains("uncited claim"));
    }

    #[test]
    fn headings_and_blank_lines_never_require_a_citation() {
        let pack = pack_with(vec![item("aaaaaaaaaaaaaaaa", "real item")]);
        let narrative =
            "# What mattered\n\n## A subsection\n\nOne cited fact [aaaaaaaaaaaaaaaa].\n";

        assert!(validate_citations(narrative, &pack).is_ok());
    }

    #[test]
    fn duplicate_citations_to_the_same_item_are_deduplicated() {
        let pack = pack_with(vec![item("aaaaaaaaaaaaaaaa", "real item")]);
        let narrative =
            "First mention [aaaaaaaaaaaaaaaa].\nSecond mention, same item [aaaaaaaaaaaaaaaa].\n";

        let citations = validate_citations(narrative, &pack).unwrap();

        assert_eq!(citations.len(), 1);
    }

    #[test]
    fn ordinary_bracketed_markdown_that_is_not_a_16_hex_id_is_not_treated_as_a_citation_token() {
        // A line like "[see the PR](url)" must not be mistaken for a
        // citation and must still fail the uncited-claim check if it makes
        // a factual assertion with no real [id] anywhere on the line.
        let pack = pack_with(vec![item("aaaaaaaaaaaaaaaa", "real item")]);
        let narrative = "See [the PR](https://example.invalid/pr/1) for details.\n";

        let rejection = validate_citations(narrative, &pack).unwrap_err();
        assert!(rejection.reason.contains("uncited claim"));
    }

    #[test]
    fn empty_narrative_trivially_passes_with_no_citations() {
        let pack = pack_with(vec![]);
        assert_eq!(validate_citations("", &pack).unwrap(), Vec::new());
    }
}

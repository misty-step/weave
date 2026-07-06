//! Plain-text cleanup for excerpts sourced from Markdown-bearing bodies
//! (Powder card comments, campaign receipt bodies): strip inline Markdown
//! syntax and truncate on word boundaries, in that order. Order matters --
//! truncating first can cut inside a `**bold**` marker or land the ellipsis
//! mid-word, which is exactly the bug this module exists to fix (found live
//! via a designer critique of fleet-retro's daily report: a Powder comment
//! excerpt rendered literal `\n\n` and a mid-word `**Delivery = two-phas…`).

/// Strip common inline Markdown syntax (bold/italic/code markers) and
/// collapse all whitespace -- including the literal blank-line paragraph
/// breaks a comment or receipt body carries -- onto a single line. Not a
/// full Markdown parser: good enough for a one-line excerpt where
/// formatting is never rendered, only ever displayed as plain text.
pub fn plain_text(raw: &str) -> String {
    let mut cleaned = raw.to_string();
    for marker in ["***", "**", "__", "`"] {
        cleaned = cleaned.replace(marker, "");
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Truncate already-plain text to at most `budget` characters, breaking on
/// a word boundary (never mid-word) and appending an ellipsis only when
/// something was actually cut. Falls back to a hard char cut only when a
/// single word alone exceeds the whole budget, so this never panics on a
/// pathological no-space input and never returns an empty string for a
/// non-empty one.
pub fn truncate_words(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let mut out = String::new();
    for word in text.split(' ') {
        let sep = if out.is_empty() { 0 } else { 1 };
        if out.chars().count() + sep + word.chars().count() > budget {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        out = text.chars().take(budget).collect();
    }
    format!("{out}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_strips_bold_markers_and_collapses_paragraph_breaks() {
        let raw = "SHAPED WITH OPERATOR morning\n\n**Delivery = two-phased**, ship then harden.";
        let cleaned = plain_text(raw);
        assert!(!cleaned.contains("**"));
        assert!(!cleaned.contains('\n'));
        assert!(cleaned.contains("Delivery = two-phased, ship then harden."));
    }

    #[test]
    fn truncate_words_never_cuts_mid_word() {
        let text = "Delivery = two-phased, ship then harden the design.";
        let truncated = truncate_words(text, 20);
        assert!(truncated.ends_with('…'));
        let body = truncated.trim_end_matches('…');
        for word in body.split_whitespace() {
            assert!(
                text.split_whitespace().any(|w| w == word),
                "{word} was not a whole word from the source text"
            );
        }
    }

    #[test]
    fn truncate_words_is_a_no_op_under_budget() {
        assert_eq!(truncate_words("short text", 100), "short text");
    }

    #[test]
    fn truncate_words_hard_cuts_a_single_word_exceeding_the_whole_budget() {
        let truncated = truncate_words("supercalifragilisticexpialidocious", 10);
        assert!(truncated.ends_with('…'));
        assert_eq!(truncated.chars().count(), 11);
    }
}

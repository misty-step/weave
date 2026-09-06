//! Plain-text cleanup for excerpts sourced from Markdown-bearing receipt bodies.

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
}

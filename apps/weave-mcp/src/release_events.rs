use serde_json::Value;

const DEFAULT_BASE_URL: &str = "https://weave-release-events.mistystep.io";

/// `RELEASE_EVENTS_READER_TOKEN` from the environment, falling back to
/// `~/.secrets` (same pattern `fleet-retro`'s `secrets.rs` uses, and
/// bridge.py's `publish_to_shelf` before that) so an MCP client launched
/// without an interactively-sourced shell environment still finds it.
/// Never printed, never embedded in a tool result.
fn reader_token() -> Option<String> {
    if let Ok(token) = std::env::var("RELEASE_EVENTS_READER_TOKEN")
        && !token.trim().is_empty()
    {
        return Some(token);
    }
    let home = std::env::var("HOME").ok()?;
    let path = std::path::Path::new(&home).join(".secrets");
    let contents = std::fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("export RELEASE_EVENTS_READER_TOKEN=") {
            let token = rest.trim().trim_matches('"');
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

fn base_url() -> String {
    std::env::var("WEAVE_RELEASE_EVENTS_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

/// Query the deployed release-events receiver (`apps/release-events`) for
/// stored Landmark webhook/release-kit events, optionally since an RFC3339
/// timestamp. Returns the receiver's own `{"events": [...]}` JSON body
/// unmodified -- this tool is a thin read-through, not a reinterpretation of
/// the receiver's schema.
pub fn list_release_events(since: Option<&str>) -> Result<Value, String> {
    let Some(token) = reader_token() else {
        return Err("RELEASE_EVENTS_READER_TOKEN not set (checked env and ~/.secrets)".to_string());
    };
    let mut url = format!("{}/v1/events", base_url());
    if let Some(since) = since {
        url = format!("{url}?since={}", urlencode(since));
    }
    let response = ureq::get(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| format!("release-events request failed: {err}"))?;
    response
        .into_json::<Value>()
        .map_err(|err| format!("release-events response was not valid JSON: {err}"))
}

fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_escapes_rfc3339_special_characters() {
        assert_eq!(
            urlencode("2026-07-04T21:00:00Z"),
            "2026-07-04T21%3A00%3A00Z"
        );
    }

    #[test]
    fn urlencode_leaves_safe_characters_untouched() {
        assert_eq!(urlencode("abc-DEF_123.~"), "abc-DEF_123.~");
    }

    #[test]
    fn default_receiver_is_the_canonical_digitalocean_endpoint() {
        assert_eq!(
            DEFAULT_BASE_URL,
            "https://weave-release-events.mistystep.io"
        );
    }
}

/// Shared "read a var from the environment, falling back to `~/.secrets`"
/// lookup. LaunchAgents do not inherit the interactive shell's sourced
/// `~/.secrets` (launchd starts a fresh minimal environment), so any value
/// this crate needs at 21:00 unattended must fall back to reading the file
/// directly -- the same fallback bridge.py's `publish_to_shelf` already
/// relies on for `ARTIFACTS_API_TOKEN`. Values are only ever used as
/// request-header/query material, never printed or embedded in generated
/// output.
pub fn env_or_secrets_file(name: &str) -> Option<String> {
    if let Ok(value) = std::env::var(name)
        && !value.trim().is_empty()
    {
        return Some(value);
    }
    let home = std::env::var("HOME").ok()?;
    let path = std::path::Path::new(&home).join(".secrets");
    let contents = std::fs::read_to_string(path).ok()?;
    let prefix = format!("export {name}=");
    for line in contents.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&prefix) {
            let value = rest.trim().trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_secrets_file_when_env_var_absent() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join(".secrets"),
            "export SOME_OTHER_VAR=\"nope\"\nexport FLEET_RETRO_TEST_TOKEN=\"abc123\"\n",
        )
        .unwrap();
        // SAFETY: single-threaded test, scoped env mutation restored after.
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::remove_var("FLEET_RETRO_TEST_TOKEN");
            std::env::set_var("HOME", home.path());
        }

        let value = env_or_secrets_file("FLEET_RETRO_TEST_TOKEN");

        unsafe {
            match &original_home {
                Some(h) => std::env::set_var("HOME", h),
                None => std::env::remove_var("HOME"),
            }
        }
        assert_eq!(value.as_deref(), Some("abc123"));
    }

    #[test]
    fn prefers_env_var_over_secrets_file() {
        // SAFETY: single-threaded test, scoped env mutation.
        unsafe {
            std::env::set_var("FLEET_RETRO_TEST_TOKEN_2", "from-env");
        }
        let value = env_or_secrets_file("FLEET_RETRO_TEST_TOKEN_2");
        unsafe {
            std::env::remove_var("FLEET_RETRO_TEST_TOKEN_2");
        }
        assert_eq!(value.as_deref(), Some("from-env"));
    }
}

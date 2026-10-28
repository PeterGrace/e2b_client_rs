//! The MCP gateway that SDK 2.61 starts in a sandbox created with an MCP
//! configuration (`e2b/sandbox_async/main.py:244-268`): see
//! [`crate::CreateSandboxParams::mcp`], [`crate::Sandbox::mcp_url`] and
//! [`crate::Sandbox::mcp_token`].

use std::borrow::Cow;

use serde_json::{Map, Value};

use crate::error::Result;

/// The port the MCP gateway listens on inside the sandbox
/// (`e2b/sandbox/main.py:28`).
pub const MCP_PORT: u16 = 50005;

/// The template used when an MCP configuration is given without a template
/// (`e2b/sandbox/main.py:33`).
pub const DEFAULT_MCP_TEMPLATE: &str = "mcp-gateway";

/// Where the gateway keeps its token (`e2b/sandbox_async/main.py:1063-1073`).
pub(crate) const MCP_TOKEN_PATH: &str = "/etc/mcp-gateway/.token";

/// The environment variable the gateway reads its token from.
pub(crate) const MCP_TOKEN_ENV: &str = "GATEWAY_ACCESS_TOKEN";

/// The user the gateway runs as, and its token file is read as.
pub(crate) const MCP_USER: &str = "root";

/// The command line that starts the gateway, `mcp-gateway --config '<json>'`,
/// as both SDKs build it.
///
/// The JSON is compact (`{"fetch":{}}`), as the JS SDK's `JSON.stringify`
/// writes it; Python's `json.dumps` puts a space after `:` and `,`. Both are
/// the same JSON to the gateway.
///
/// # Arguments
///
/// * `config` - the MCP server configuration.
///
/// # Returns
///
/// The shell command line.
///
/// # Errors
///
/// [`crate::Error::Decode`] if the configuration cannot be serialized. A
/// `Map<String, Value>` always can, so this does not happen in practice.
pub(crate) fn gateway_command(config: &Map<String, Value>) -> Result<String> {
    let json = serde_json::to_string(config)?;
    Ok(format!("mcp-gateway --config {}", shell_quote(&json)))
}

/// POSIX shell quoting, as Python's `shlex.quote` and the JS SDK's
/// `shellQuote` (`dist/index.js:268, 277-281`) do: the empty string is
/// `''`, a string of only ASCII letters, digits and `_@%+=:,./-` stays as it
/// is, and anything else is single-quoted with each `'` written as `'"'"'`.
///
/// Rust note: `Cow` (clone on write) hands back the input itself when no
/// quoting is needed, and allocates a new `String` only when it is.
fn shell_quote(raw: &str) -> Cow<'_, str> {
    if raw.is_empty() {
        return Cow::Borrowed("''");
    }
    let safe = raw
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte));
    if safe {
        Cow::Borrowed(raw)
    } else {
        Cow::Owned(format!("'{}'", raw.replace('\'', r#"'"'"'"#)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_matches_shlex_quote() {
        assert_eq!(shell_quote(""), "''");
        assert_eq!(shell_quote("abc-1.2/x:y,z@%+=_"), "abc-1.2/x:y,z@%+=_");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), r#"'it'"'"'s'"#);
        // Non-ASCII letters are unsafe, as `shlex.quote`'s `re.ASCII` `\w`.
        assert_eq!(shell_quote("caf\u{e9}"), "'caf\u{e9}'");
        assert_eq!(shell_quote("$HOME"), "'$HOME'");
    }

    #[test]
    fn shell_quote_borrows_a_safe_string() {
        assert!(matches!(shell_quote("safe"), Cow::Borrowed("safe")));
    }

    #[test]
    fn gateway_command_quotes_the_json_config() {
        let serde_json::Value::Object(config) = serde_json::json!({"fetch": {}}) else {
            panic!("an object");
        };
        assert_eq!(
            gateway_command(&config).expect("command"),
            r#"mcp-gateway --config '{"fetch":{}}'"#
        );
    }

    #[test]
    fn gateway_command_escapes_a_single_quote_in_the_config() {
        let serde_json::Value::Object(config) = serde_json::json!({"x": {"arg": "it's"}}) else {
            panic!("an object");
        };
        assert_eq!(
            gateway_command(&config).expect("command"),
            r#"mcp-gateway --config '{"x":{"arg":"it'"'"'s"}}'"#
        );
    }
}

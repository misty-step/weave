#![forbid(unsafe_code)]

use serde_json::{Value, json};

mod fleet_retro;
mod release_events;

/// Hand-rolled JSON-RPC 2.0 stdio server, no external MCP SDK dependency --
/// the same shape `powder-mcp` uses (`crates/powder-mcp/src/lib.rs` in the
/// powder repo), named directly as the fleet reference shape for
/// MCP-over-existing-core. weave-mcp is read-only by design: every tool
/// either queries an existing HTTP source (release-events) or triggers a
/// local, non-publishing fleet-retro dry-run / reads an already-published
/// spec.json off disk. No tool here can write to Powder, publish to the
/// shelf, or post to the Bridge feed -- those stay CLI/LaunchAgent actions
/// until an operator explicitly signs off on MCP-driven publication
/// (mirroring bitterblossom's MCP-dispatch-off-by-default, which the
/// misty-step-915 audit specifically praised as a deliberate, named
/// exception rather than a gap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: &'static str,
}

pub const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "list_release_events",
        description: "Query the deployed release-events receiver (apps/release-events) for stored Landmark webhook/release-kit events, optionally since an RFC3339 timestamp.",
        input_schema: r#"{"type":"object","properties":{"since":{"type":"string","description":"RFC3339 timestamp; only events received after this are returned"}}}"#,
    },
    ToolDef {
        name: "run_fleet_retro",
        description: "Trigger a fleet-retro assembly run (daily/weekly/custom window) and return the assembled RetroSpec as JSON. Always dry-run: never publishes to the shelf or posts a Bridge feed entry.",
        input_schema: r#"{"type":"object","properties":{"window":{"type":"string","enum":["daily","weekly","custom"],"default":"daily"},"since":{"type":"string","description":"RFC3339; required when window=custom"},"until":{"type":"string","description":"RFC3339; defaults to now when window=custom"},"bb_plane":{"type":"string","description":"optional bb plane.toml directory to include Bitterblossom run history"}}}"#,
    },
    ToolDef {
        name: "get_latest_fleet_retro",
        description: "Read the most recently published fleet-retro spec.json from ~/.factory-lanes/fleet-retro, optionally filtered to one window (daily/weekly). Reflects the last real publish, not a fresh assembly.",
        input_schema: r#"{"type":"object","properties":{"window":{"type":"string","enum":["daily","weekly"]}}}"#,
    },
];

pub fn tools() -> &'static [ToolDef] {
    TOOLS
}

pub fn tool_defs_json() -> Value {
    Value::Array(
        TOOLS
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": serde_json::from_str::<Value>(tool.input_schema)
                        .expect("tool schema is valid json"),
                })
            })
            .collect(),
    )
}

/// Dispatch one JSON-RPC 2.0 request line. Returns `None` for notifications
/// (requests without an `id`), matching the spec -- no response is sent for
/// those.
pub fn handle_json_rpc(request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": request["params"]["protocolVersion"]
                .as_str()
                .unwrap_or("2024-11-05"),
            "serverInfo": {"name": "weave", "version": env!("CARGO_PKG_VERSION")},
            "capabilities": {"tools": {"listChanged": false}},
        })),
        "tools/list" => Ok(json!({ "tools": tool_defs_json() })),
        "tools/call" => {
            let params = &request["params"];
            let name = params["name"].as_str().unwrap_or("");
            let args = &params["arguments"];
            call_tool(name, args)
        }
        "ping" => Ok(json!({})),
        other => Err(format!("method not found: {other}")),
    };

    id.map(|id| match result {
        Ok(value) => json!({"jsonrpc": "2.0", "id": id, "result": value}),
        Err(message) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32603, "message": message},
        }),
    })
}

fn fleet_retro_dir() -> std::path::PathBuf {
    std::env::var("FLEET_RETRO_OUT_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".factory-lanes")
                .join("fleet-retro")
        })
}

pub fn call_tool(name: &str, args: &Value) -> Result<Value, String> {
    let payload = match name {
        "list_release_events" => {
            let since = args["since"].as_str();
            release_events::list_release_events(since)?
        }
        "run_fleet_retro" => {
            let window = args["window"].as_str().unwrap_or("daily");
            let since = args["since"].as_str();
            let until = args["until"].as_str();
            let bb_plane = args["bb_plane"].as_str();
            fleet_retro::run_fleet_retro(window, since, until, bb_plane)?
        }
        "get_latest_fleet_retro" => {
            let window = args["window"].as_str();
            fleet_retro::get_latest_fleet_retro(&fleet_retro_dir(), window)?
        }
        other => return Err(format!("unknown tool: {other}")),
    };
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&payload).unwrap_or_default()}],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_reports_server_info_and_tool_capability() {
        let response = handle_json_rpc(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
        }))
        .unwrap();
        assert_eq!(response["result"]["serverInfo"]["name"], "weave");
        assert_eq!(
            response["result"]["capabilities"]["tools"]["listChanged"],
            false
        );
    }

    #[test]
    fn tools_list_returns_all_three_tool_defs() {
        let response = handle_json_rpc(&json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
        }))
        .unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"list_release_events"));
        assert!(names.contains(&"run_fleet_retro"));
        assert!(names.contains(&"get_latest_fleet_retro"));
    }

    #[test]
    fn unknown_method_returns_a_jsonrpc_error() {
        let response = handle_json_rpc(&json!({
            "jsonrpc": "2.0", "id": 3, "method": "not_a_real_method", "params": {}
        }))
        .unwrap();
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("method not found")
        );
    }

    #[test]
    fn notification_without_id_returns_none() {
        let response = handle_json_rpc(&json!({
            "jsonrpc": "2.0", "method": "ping", "params": {}
        }));
        assert!(response.is_none());
    }

    #[test]
    fn unknown_tool_call_returns_an_error_result() {
        let response = handle_json_rpc(&json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {"name": "not_a_real_tool", "arguments": {}}
        }))
        .unwrap();
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("unknown tool")
        );
    }

    // get_latest_fleet_retro's missing-directory error path is covered by
    // fleet_retro::tests::get_latest_fleet_retro_errors_clearly_when_nothing_matches,
    // which exercises it directly against an explicit path rather than via
    // process-global FLEET_RETRO_OUT_ROOT env mutation -- this crate
    // `#![forbid(unsafe_code)]`, so no test here reaches for `set_var`.
}

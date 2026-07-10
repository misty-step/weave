---
name: weave-mcp
description: |
  Query weave's own data surfaces (fleet-retro agent-activity reports,
  release-events webhook/release-kit history) over MCP instead of shelling
  into the CLI or reading raw JSONL/HTML. Use when an agent needs "what did
  the fleet do recently", "latest fleet retro", "recent release events",
  "trigger a retro for this window", or is operating in weave and needs its
  own tool-call surface rather than ad hoc shell commands.
---

# weave-mcp

`apps/weave-mcp` is a hand-rolled JSON-RPC 2.0 stdio MCP server (no external
MCP SDK dependency -- same shape as `powder-mcp`, the fleet's reference
implementation for MCP-over-existing-core). Read-only by design: every tool
either queries an existing HTTP source, triggers a local non-publishing
dry-run, or reads an already-published file off disk. No tool here writes to
Powder, publishes to the bastion shelf, or posts to the Bridge feed --
publication stays a CLI/LaunchAgent action.

## Run it

```bash
cargo run --release -p weave-mcp
```

Reads/writes JSON-RPC 2.0 request/response objects, one per line, over
stdin/stdout. Register it with any MCP-capable client the same way you'd
register `powder-mcp` or `bb --config <plane> mcp serve`.

## Tools

| Tool | Verb | Notes |
|---|---|---|
| `list_release_events` | query | Reads the canonical DigitalOcean `apps/release-events` receiver's `GET /v1/events?since=` -- Landmark webhook + release-kit events. Requires `RELEASE_EVENTS_READER_TOKEN` (env or `~/.secrets`). |
| `run_fleet_retro` | trigger (dry-run only) | Assembles a fresh `RetroSpec` for `window: daily\|weekly\|custom` and returns it as JSON. Never publishes -- no shelf write, no feed post. Use for "what would today's/this week's/this window's retro look like right now." |
| `get_latest_fleet_retro` | read | Reads the most recently *published* `spec.json` under `~/.factory-lanes/fleet-retro/` (optionally filtered to `daily`/`weekly`). Reflects the last real CLI/LaunchAgent run, not a fresh assembly. Use for "what did the last daily/weekly retro actually say." |

`run_fleet_retro` vs `get_latest_fleet_retro`: the first computes fresh
evidence right now (any arbitrary window, nothing persisted); the second
reads what was already generated and published (only the standard
daily/weekly cadence, whatever ran last).

## Secrets

`RELEASE_EVENTS_READER_TOKEN` is read from the environment first, falling
back to `~/.secrets` -- required because an MCP client launched outside an
interactively-sourced shell (a LaunchAgent, a fresh terminal, a remote
sprite) does not inherit it. Never printed, never embedded in a tool result.
If `list_release_events` errors with "not set", the reader token has not been
mirrored into this machine's `~/.secrets` -- that is an operator provisioning
gap, not a code bug. Provider secret inventories confirm names only and must
never be used to print or copy values into a receipt.

## Related

- `docs/fleet-retro.md` — the fleet-retro generator this MCP server reads
  from and triggers.
- `docs/release-event-receiver.md` — the release-events receiver this MCP
  server queries.
- Powder's `powder-mcp` (`crates/powder-mcp` in the powder repo) — the fleet
  reference shape this server's JSON-RPC dispatch mirrors.

## Red lines

- No tool here mutates Powder, publishes to the shelf, or posts to the
  Bridge feed. If a future card wants MCP-driven publication, that needs its
  own explicit operator sign-off and a new tool, not a flag on
  `run_fleet_retro` -- mirroring bitterblossom's MCP-dispatch-off-by-default
  caution.
- Never print `RELEASE_EVENTS_READER_TOKEN`, `POWDER_API_KEY`, or
  `ARTIFACTS_API_TOKEN` in a tool result, log line, or error message.

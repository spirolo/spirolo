# Use Spirolo from an MCP client

The Model Context Protocol (MCP) server lets an editor or agent inspect a project
through the installed Spirolo binary. Install the binary first, then configure your
client to start it as a local process. No hosted Spirolo service is required.

Configure an MCP client to launch the installed binary with `args: ["mcp"]`:

```json
{
  "mcpServers": {
    "spirolo": {
      "command": "spirolo",
      "args": ["mcp"]
    }
  }
}
```

The server reads one JSON-RPC 2.0 message per line on stdin and flushes each reply to
stdout. Logs go to stderr. It supports initialization, `tools/list`, `tools/call`, and
`ping`; notifications receive no reply. Closing stdin stops the server.

| Tool | Arguments | CLI JSON counterpart |
|---|---|---|
| `check` | Optional `root`, `config`, `rule`, `all` | `spirolo check <root> --format json [--config <config>] [--rule <rule>] [--all]` |
| `explain` | Required `query` (finding id or rule name); optional `root`, `config` | `spirolo explain <query> <root> --format json [--config <config>]` |
| `impact` | Required `files` (array of changed paths); optional `root` | `spirolo impact <files...> --root <root> --format json` |
| `list` | Optional `root` | `spirolo list <root> --format json` |
| `rule_status` | None | `spirolo rule-status --format json` |

`root` defaults to the server's current directory; `path` is also accepted as its alias.
Changed paths are relative to that root. Config paths have the CLI's resolution rules.
Each successful tool result contains one MCP text block whose text is the CLI's JSON
document. Findings that would fail a CLI gate still return a successful analysis result.
The check result uses the CLI's baseline filter and [rule-tier policy](tiers.md),
including applicable severity overrides (`all` includes hidden findings). Each
finding carries `rule_status`, the rule's verdict in the embedded status table (`PASS`, `UNADJUDICATED`, `INSUFFICIENT`, `SKIPPED`,
`UNMEASURED`, `FAIL` or `DROPPED`). Explanation looks up ids before that
filter. The status document is the complete embedded measurement table.

Unknown methods return JSON-RPC error `-32601`. Unknown tools and invalid tool arguments
return `-32602`; malformed JSON returns `-32700`, and invalid requests return `-32600`.
Analysis failures, such as a missing project or unknown finding id, return a text block
with `isError: true`. These errors leave the server available for the next request.

The server supports protocol versions `2024-11-05`, `2025-03-26`, and `2025-06-18`.
If the client requests another version, initialization offers `2025-06-18`.
Transport and tool results follow the
[MCP stdio specification](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
and [tool specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools).

Each call builds a fresh model in process, so later calls see project changes. The server
reads projects and existing artifacts, without executing tests or spawning CLI commands.

The tool reports evidence from static analysis and supplied artifacts. It does not
prove that a test passes or that a suggested repair works. Read each rule's status
and limits before acting on a finding. Protocol versions and error codes above
come from the [server implementation](../crates/spirolo/src/mcp.rs).

//! MCP's newline-delimited JSON-RPC stdio transport; analysis stays in process.

use std::io::{BufRead as _, Write as _};
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{json, Value};

/// Serve until stdin closes. Stdout contains only newline-delimited JSON-RPC messages.
pub fn serve() -> u8 {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let response = match line {
            Ok(line) => serde_json::from_str::<Value>(&line).map_or_else(
                |_| Some(error(&Value::Null, -32700, "Parse error")),
                |request| respond(&request),
            ),
            Err(error) => {
                eprintln!("spirolo mcp: cannot read stdin: {error}");
                return super::EXIT_ERROR;
            }
        };
        if let Some(response) = response {
            if let Err(error) = writeln!(stdout, "{response}").and_then(|()| stdout.flush()) {
                if error.kind() == std::io::ErrorKind::BrokenPipe {
                    return super::EXIT_OK;
                }
                eprintln!("spirolo mcp: cannot write stdout: {error}");
                return super::EXIT_ERROR;
            }
        }
    }
    super::EXIT_OK
}

fn respond(request: &Value) -> Option<Value> {
    let valid_id = request
        .get("id")
        .is_none_or(|id| id.is_null() || id.is_string() || id.is_i64() || id.is_u64());
    if request["jsonrpc"] != "2.0" || !request["method"].is_string() || !valid_id {
        let id = if valid_id {
            request["id"].clone()
        } else {
            Value::Null
        };
        return Some(error(&id, -32600, "Invalid Request"));
    }
    let id = request.get("id")?.clone();
    let protocol = request["params"]["protocolVersion"]
        .as_str()
        .filter(|version| matches!(*version, "2024-11-05" | "2025-03-26" | "2025-06-18"))
        .unwrap_or("2025-06-18");
    let result = match request["method"].as_str() {
        Some("initialize") => json!({
            "protocolVersion":protocol, "capabilities":{"tools":{}},
            "serverInfo":{"name":"spirolo", "version":super::VERSION}
        }),
        Some("tools/list") => json!({"tools":tools()}),
        Some("ping") => json!({}),
        Some("tools/call") => match call(&request["params"]) {
            Ok(result) => result,
            Err(message) => return Some(error(&id, -32602, &message)),
        },
        _ => return Some(error(&id, -32601, "Method not found")),
    };
    Some(json!({"jsonrpc":"2.0", "id":id, "result":result}))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckInput {
    #[serde(alias = "path")]
    root: Option<PathBuf>,
    config: Option<PathBuf>,
    rule: Option<String>,
    /// Every rule's findings, as `check --all`; otherwise only those of rules shown by default.
    #[serde(default)]
    all: bool,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootInput {
    #[serde(alias = "path")]
    root: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExplainInput {
    #[serde(alias = "path")]
    root: Option<PathBuf>,
    query: String,
    config: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImpactInput {
    #[serde(alias = "path")]
    root: Option<PathBuf>,
    files: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusInput {}

fn call(params: &Value) -> Result<Value, String> {
    let name = params["name"].as_str().ok_or("expected a tool name")?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let rendered = match name {
        "check" => {
            let input: CheckInput = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            check(&input)
        }
        "explain" => {
            let input: ExplainInput =
                serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            explain(&input)
        }
        "impact" => {
            let input: ImpactInput =
                serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            impact(&input)
        }
        "list" => {
            let input: RootInput = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            list(&input)
        }
        "rule_status" => {
            let _: StatusInput = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
            Ok(spirolo_rules::status::json().to_owned())
        }
        _ => return Err(format!("unknown tool '{name}'")),
    };
    Ok(match rendered {
        Ok(text) => json!({"content":[{"type":"text", "text":text}]}),
        Err(message) => json!({"isError":true, "content":[{"type":"text", "text":message}]}),
    })
}

fn list(input: &RootInput) -> Result<String, String> {
    let (model, _) = super::build_list(input.root.as_deref())?;
    Ok(spirolo_report::to_json(&model))
}

fn impact(input: &ImpactInput) -> Result<String, String> {
    let root = super::resolve_root(input.root.as_deref())?;
    Ok(spirolo_report::impact_to_json(&super::build_impact(
        &root,
        &input.files,
    )))
}

fn explain(input: &ExplainInput) -> Result<String, String> {
    super::render_explain(&super::ExplainArgs {
        query: input.query.clone(),
        path: input.root.clone(),
        config: input.config.clone(),
        format: super::ExplainFormat::Json,
    })
}

fn check(input: &CheckInput) -> Result<String, String> {
    super::render_check(
        &super::CheckArgs {
            path: input.root.clone(),
            format: super::Format::Json,
            agent: false,
            config: input.config.clone(),
            rule: input.rule.clone(),
            all: input.all,
            fail_on_stale_baseline: false,
            churn: None,
        },
        None,
        false,
    )
    .map(|(rendered, _)| rendered)
}

fn tools() -> Value {
    let root = json!({"type":"string", "description":"Project root; defaults to the server's current directory."});
    json!([
        {"name":"check", "description":"Run enabled rules; return check --format json, only rules shown by default unless all is true. Each finding carries rule_status, its rule's measured verdict (PASS, UNADJUDICATED, INSUFFICIENT, SKIPPED, UNMEASURED, FAIL or DROPPED).",
         "inputSchema":{"type":"object", "properties":{"root":root,"config":{"type":"string"},"rule":{"type":"string"},"all":{"type":"boolean"}}}},
        {"name":"explain", "description":"Explain a finding id or rule name; return explain --format json.",
         "inputSchema":{"type":"object", "properties":{"root":root,"query":{"type":"string"},"config":{"type":"string"}}, "required":["query"]}},
        {"name":"impact", "description":"Rank tests reached by changed files; return impact --format json.",
         "inputSchema":{"type":"object", "properties":{"root":root,"files":{"type":"array","items":{"type":"string"}}}, "required":["files"]}},
        {"name":"list", "description":"Enumerate the test graph; return list --format json.",
         "inputSchema":{"type":"object", "properties":{"root":root}}},
        {"name":"rule_status", "description":"Return the embedded measurement table as rule-status --format json.",
         "inputSchema":{"type":"object", "properties":{}}}
    ])
}

fn error(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0", "id":id, "error":{"code":code, "message":message}})
}

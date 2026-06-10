//! MCP(Model Context Protocol) 서버 — 원장 도구를 외부 클라이언트에 노출.
//!
//! CLI 위임 백엔드(Claude Code 등)는 자기 도구만 쓸 수 있어 원장의 비서 도구
//! (기억·스킬·알림·할일·가계부·실시간 정보)가 닿지 않았다. 이 서버를
//! `--mcp-config`로 물려주면 키 없이 위임하는 사용자도 같은 도구를 쓴다.
//!
//! 전송: stdio 위 JSON-RPC 2.0, 줄 단위 구분(mcp.rs 클라이언트와 동일 프레이밍).
//! 주의: 프로토콜이 stdout을 쓰므로 이 모드에선 stdout에 다른 출력 금지.

use crate::tools::{Tool, ToolContext};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// MCP로 노출하는 도구 모음 — 원장 고유 비서 도구만.
/// 셸·파일·웹·클립보드·서브에이전트는 제외: 클라이언트(Claude Code)가 자체 도구로
/// 갖고 있고, MCP로 또 열면 클라이언트의 권한 정책(읽기전용 등)을 우회하게 된다.
pub fn served_tools() -> Vec<Box<dyn Tool>> {
    use crate::tools::*;
    vec![
        Box::new(memory::RememberTool),
        Box::new(memory::RecallTool),
        Box::new(skill::SaveSkillTool),
        Box::new(skill::ListSkillsTool),
        Box::new(skill::ReadSkillTool),
        Box::new(notes::NoteSearchTool),
        Box::new(notes::NoteReadTool),
        Box::new(notes::NoteAppendTool),
        Box::new(notes::NoteListTool),
        Box::new(reminder::AddReminderTool),
        Box::new(reminder::ListRemindersTool),
        Box::new(reminder::RemoveReminderTool),
        Box::new(todo::AddTodoTool),
        Box::new(todo::ListTodosTool),
        Box::new(todo::CompleteTodoTool),
        Box::new(dday::AddDdayTool),
        Box::new(dday::ListDdaysTool),
        Box::new(notion::NotionSearchTool),
        Box::new(notion::NotionAppendTool),
        Box::new(expense::AddExpenseTool),
        Box::new(expense::ExpenseSummaryTool),
        Box::new(habit::AddHabitTool),
        Box::new(habit::CheckHabitTool),
        Box::new(habit::ListHabitsTool),
        Box::new(subway::SubwayTool),
        Box::new(weather::WeatherTool),
        Box::new(airquality::AirQualityTool),
        Box::new(exchange::ExchangeTool),
        Box::new(coin::CoinTool),
        Box::new(news::NewsTool),
        Box::new(lotto::LottoTool),
    ]
}

/// 요청 한 줄을 처리해 응답 JSON(한 줄)을 돌려준다. 알림(id 없음)은 None.
/// 순수 함수에 가깝게 유지해 단위 테스트를 가능하게 한다.
pub fn handle_line(line: &str, tools: &[Box<dyn Tool>], ctx: &ToolContext) -> Option<String> {
    let req: Value = match serde_json::from_str(line.trim()) {
        Ok(v) => v,
        Err(_) => {
            return Some(
                json!({"jsonrpc":"2.0","id":Value::Null,
                       "error":{"code":-32700,"message":"parse error"}})
                .to_string(),
            )
        }
    };
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    // 알림(id 없음)은 응답하지 않는다(notifications/initialized 등).
    let id = match id {
        Some(id) if !id.is_null() => id,
        _ => return None,
    };

    let result: Result<Value, (i64, String)> = match method {
        "initialize" => {
            // 클라이언트가 요청한 프로토콜 버전을 그대로 돌려준다(없으면 기준 버전).
            let ver = req
                .pointer("/params/protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05");
            Ok(json!({
                "protocolVersion": ver,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "wonjang", "version": env!("CARGO_PKG_VERSION") }
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => {
            let list: Vec<Value> = tools
                .iter()
                .map(|t| {
                    let spec = t.spec();
                    json!({
                        "name": spec.name,
                        "description": spec.description,
                        "inputSchema": spec.parameters,
                    })
                })
                .collect();
            Ok(json!({ "tools": list }))
        }
        "tools/call" => {
            let name = req
                .pointer("/params/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = req
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match tools.iter().find(|t| t.name() == name) {
                None => Err((-32602, format!("알 수 없는 도구: {name}"))),
                Some(tool) => {
                    // 도구 실행 오류는 MCP 관례대로 isError 결과로(프로토콜 오류 아님).
                    let (text, is_error) = match tool.execute(&args, ctx) {
                        Ok(t) => (t, false),
                        Err(e) => (format!("도구 실행 오류: {e:#}"), true),
                    };
                    Ok(json!({
                        "content": [ { "type": "text", "text": text } ],
                        "isError": is_error
                    }))
                }
            }
        }
        other => Err((-32601, format!("지원하지 않는 메서드: {other}"))),
    };

    let resp = match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err((code, message)) => {
            json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
        }
    };
    Some(resp.to_string())
}

/// stdio에서 MCP 서버를 돌린다(클라이언트가 파이프를 닫을 때까지).
pub fn serve() -> anyhow::Result<()> {
    let tools = served_tools();
    // 노출 도구에 셸이 없으므로 승인 정책은 사실상 무의미하지만, 보수적으로 둔다.
    let ctx = ToolContext {
        auto_approve: false,
        allow_dangerous: false,
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(resp) = handle_line(&line, &tools, &ctx) {
            stdout.write_all(resp.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ToolContext {
        ToolContext {
            auto_approve: false,
            allow_dangerous: false,
        }
    }

    #[test]
    fn initialize_echoes_protocol_and_names_server() {
        let tools = served_tools();
        let resp = handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#,
            &tools,
            &ctx(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(v["result"]["serverInfo"]["name"], "wonjang");
    }

    #[test]
    fn notifications_get_no_response() {
        let tools = served_tools();
        assert!(handle_line(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &tools,
            &ctx()
        )
        .is_none());
    }

    #[test]
    fn tools_list_exposes_assistant_tools_but_never_shell_or_fs() {
        let tools = served_tools();
        let resp = handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            &tools,
            &ctx(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        let names: Vec<&str> = v["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        for must in ["remember", "save_skill", "add_reminder", "weather_now"] {
            assert!(names.contains(&must), "'{must}' 도구가 노출 안 됨");
        }
        // 클라이언트 권한 정책 우회 방지: 셸·파일·웹·클립보드·spawn 금지.
        for banned in [
            "run_shell",
            "read_file",
            "write_file",
            "list_dir",
            "web_search",
            "web_fetch",
            "read_clipboard",
            "write_clipboard",
            "spawn_subagent",
        ] {
            assert!(!names.contains(&banned), "'{banned}'가 MCP로 새어나감");
        }
    }

    #[test]
    fn tools_call_runs_pure_tool_and_unknown_is_error() {
        let tools = served_tools();
        // 로또는 순수(저장소 안 건드림) — 실행 경로 검증에 적합.
        let resp = handle_line(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"lotto_numbers","arguments":{"games":1}}}"#,
            &tools,
            &ctx(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], false);
        assert!(v["result"]["content"][0]["text"].as_str().unwrap().len() > 5);

        let resp = handle_line(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"없는도구"}}"#,
            &tools,
            &ctx(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"]["code"], -32602);
    }

    #[test]
    fn unknown_method_and_parse_error_are_jsonrpc_errors() {
        let tools = served_tools();
        let v: Value = serde_json::from_str(
            &handle_line(r#"{"jsonrpc":"2.0","id":5,"method":"x/y"}"#, &tools, &ctx()).unwrap(),
        )
        .unwrap();
        assert_eq!(v["error"]["code"], -32601);
        let v: Value =
            serde_json::from_str(&handle_line("{깨진 json", &tools, &ctx()).unwrap()).unwrap();
        assert_eq!(v["error"]["code"], -32700);
    }
}

//! MCP(Model Context Protocol) 클라이언트 — 외부 도구 서버 연동.
//!
//! 설정된 MCP 서버를 자식 프로세스로 띄우고 stdio 위에서 JSON-RPC 2.0(줄 단위
//! 구분)로 통신한다. 서버가 제공하는 도구 목록을 받아와 원장의 도구로 등록하면,
//! 에이전트가 외부 생태계의 도구(파일시스템 서버, 깃허브 서버 등)를 그대로 쓸 수
//! 있다.
//!
//! 동기 `Tool::execute`에서 영속 연결을 쓰기 위해, 전용 워커 스레드가 자식
//! 프로세스와 파이프를 소유하고 요청을 순차 처리한다(블로킹 std I/O).

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;

/// MCP 서버가 제공하는 도구 정의.
#[derive(Debug, Clone)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// 워커 스레드로 보내는 도구 호출 요청.
struct McpRequest {
    payload: Value,
    /// 응답(JSON-RPC result) 또는 오류 메시지를 돌려받는 채널.
    resp: Sender<Result<Value, String>>,
}

/// 연결된 MCP 서버 클라이언트.
pub struct McpClient {
    pub name: String,
    pub tools: Vec<McpToolDef>,
    tx: Mutex<Sender<McpRequest>>,
}

impl McpClient {
    /// MCP 서버를 띄우고 핸드셰이크 후 도구 목록까지 받아온다.
    pub fn connect(
        name: &str,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("MCP 서버 실행 실패: {command}"))?;

        let mut stdin = child.stdin.take().context("stdin 파이프 없음")?;
        let mut reader = BufReader::new(child.stdout.take().context("stdout 파이프 없음")?);

        // 1) initialize 핸드셰이크.
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "wonjang", "version": env!("CARGO_PKG_VERSION") }
            }
        });
        send_recv(&mut stdin, &mut reader, &init, 1)
            .context("MCP initialize 실패")?;

        // 2) initialized 알림(응답 없음).
        write_msg(
            &mut stdin,
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )?;

        // 3) tools/list.
        let list = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} });
        let result = send_recv(&mut stdin, &mut reader, &list, 2)
            .context("MCP tools/list 실패")?;
        let tools = parse_tools(&result);

        // 4) 워커 스레드: 이후 tools/call 요청을 순차 처리.
        let (tx, rx) = mpsc::channel::<McpRequest>();
        let server_name = name.to_string();
        std::thread::spawn(move || worker(child, stdin, reader, rx, server_name));

        Ok(Self {
            name: name.to_string(),
            tools,
            tx: Mutex::new(tx),
        })
    }

    /// 도구를 호출하고 결과 텍스트를 반환한다.
    pub fn call_tool(&self, tool_name: &str, args: &Value) -> Result<String> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": { "name": tool_name, "arguments": args }
        });
        let (rtx, rrx) = mpsc::channel();
        self.tx
            .lock()
            .map_err(|_| anyhow::anyhow!("MCP 채널 잠금 실패"))?
            .send(McpRequest { payload, resp: rtx })
            .map_err(|_| anyhow::anyhow!("MCP 워커가 종료되었습니다"))?;
        let result = rrx
            .recv()
            .map_err(|_| anyhow::anyhow!("MCP 응답 수신 실패"))?
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(format_tool_result(&result))
    }
}

/// 워커: 자식 프로세스와 파이프를 소유하고 요청을 순차 처리한다.
fn worker(
    _child: Child, // 드롭되면 프로세스가 죽으므로 소유만 유지.
    mut stdin: ChildStdin,
    mut reader: BufReader<ChildStdout>,
    rx: mpsc::Receiver<McpRequest>,
    server_name: String,
) {
    let mut next_id: i64 = 100;
    while let Ok(req) = rx.recv() {
        next_id += 1;
        let mut payload = req.payload;
        payload["id"] = json!(next_id);

        let result = (|| -> Result<Value, String> {
            write_msg(&mut stdin, &payload).map_err(|e| e.to_string())?;
            recv_for_id(&mut reader, next_id).map_err(|e| e.to_string())
        })();
        // 수신측이 사라졌어도 무시.
        let _ = req.resp.send(result);
    }
    let _ = server_name; // (로깅용으로 보관)
}

/// 한 줄 JSON 메시지를 쓴다(개행으로 구분).
fn write_msg(stdin: &mut ChildStdin, msg: &Value) -> Result<()> {
    let line = serde_json::to_string(msg)?;
    stdin.write_all(line.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;
    Ok(())
}

/// 메시지를 쓰고, 주어진 id의 응답을 받을 때까지 읽어 result를 반환한다.
fn send_recv(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    msg: &Value,
    id: i64,
) -> Result<Value> {
    write_msg(stdin, msg)?;
    recv_for_id(reader, id)
}

/// 지정 id의 JSON-RPC 응답을 읽는다(다른 알림/메시지는 건너뜀).
fn recv_for_id(reader: &mut BufReader<ChildStdout>, id: i64) -> Result<Value> {
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            bail!("MCP 서버가 응답 없이 연결을 종료했습니다");
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // JSON이 아닌 로그 줄은 무시.
        };
        // 우리 id의 응답인가?
        if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
            if let Some(err) = msg.get("error") {
                bail!("MCP 오류: {}", err);
            }
            return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
        }
        // 다른 알림/메시지는 건너뛴다.
    }
}

/// tools/list 결과에서 도구 정의를 파싱.
fn parse_tools(result: &Value) -> Vec<McpToolDef> {
    result
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let name = t.get("name")?.as_str()?.to_string();
                    let description = t
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let input_schema = t
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));
                    Some(McpToolDef {
                        name,
                        description,
                        input_schema,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// tools/call 결과(content 배열)를 사람이 읽을 텍스트로 합친다.
fn format_tool_result(result: &Value) -> String {
    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut parts: Vec<String> = Vec::new();
    if let Some(content) = result.get("content").and_then(|v| v.as_array()) {
        for item in content {
            match item.get("type").and_then(|v| v.as_str()) {
                Some("text") => {
                    if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                        parts.push(t.to_string());
                    }
                }
                Some(other) => parts.push(format!("[{other} 콘텐츠]")),
                None => {}
            }
        }
    }
    let body = if parts.is_empty() {
        // content가 없으면 structuredContent 등을 그대로.
        serde_json::to_string(result).unwrap_or_default()
    } else {
        parts.join("\n")
    };
    if is_error {
        format!("(도구 오류) {body}")
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tools_reads_list() {
        let result = json!({
            "tools": [
                { "name": "echo", "description": "에코", "inputSchema": { "type": "object" } },
                { "name": "add" }
            ]
        });
        let tools = parse_tools(&result);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "echo");
        assert_eq!(tools[0].description, "에코");
        assert_eq!(tools[1].name, "add");
        assert!(tools[1].description.is_empty());
    }

    #[test]
    fn format_text_content() {
        let result = json!({ "content": [ { "type": "text", "text": "안녕" }, { "type": "text", "text": "하세요" } ] });
        assert_eq!(format_tool_result(&result), "안녕\n하세요");
    }

    #[test]
    fn format_error_content() {
        let result = json!({ "isError": true, "content": [ { "type": "text", "text": "실패" } ] });
        assert_eq!(format_tool_result(&result), "(도구 오류) 실패");
    }

    // 실제 자식 프로세스(파이썬 목 서버)로 stdio JSON-RPC 전 과정을 검증.
    // 실행: `cargo test -- --ignored` (python3 필요)
    #[test]
    #[ignore]
    fn live_connect_and_call() {
        let script = r#"
import sys, json
def send(m): sys.stdout.write(json.dumps(m)+"\n"); sys.stdout.flush()
for line in sys.stdin:
    line=line.strip()
    if not line: continue
    r=json.loads(line); m=r.get("method"); i=r.get("id")
    if m=="initialize": send({"jsonrpc":"2.0","id":i,"result":{"protocolVersion":"2024-11-05","capabilities":{}}})
    elif m=="notifications/initialized": pass
    elif m=="tools/list": send({"jsonrpc":"2.0","id":i,"result":{"tools":[{"name":"greet","description":"인사","inputSchema":{"type":"object"}}]}})
    elif m=="tools/call":
        who=r["params"].get("arguments",{}).get("name","손님")
        send({"jsonrpc":"2.0","id":i,"result":{"content":[{"type":"text","text":who+"님 안녕하세요!"}]}})
"#;
        let args = vec!["-c".to_string(), script.to_string()];
        let client =
            McpClient::connect("mock", "python3", &args, &HashMap::new()).expect("연결 실패");
        assert_eq!(client.tools.len(), 1);
        assert_eq!(client.tools[0].name, "greet");
        let out = client
            .call_tool("greet", &json!({ "name": "원장" }))
            .expect("호출 실패");
        assert!(out.contains("원장님 안녕하세요"), "응답: {out}");
    }
}

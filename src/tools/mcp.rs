//! MCP 서버 도구를 원장의 `Tool`로 감싸는 어댑터.

use super::{Tool, ToolContext, ToolSpec};
use crate::mcp::McpClient;
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

/// 하나의 MCP 도구를 표현.
pub struct McpTool {
    client: Arc<McpClient>,
    /// 서버에서의 원래 도구 이름(호출 시 사용).
    mcp_name: String,
    /// 모델에 노출하는 이름(`mcp_<server>_<tool>`), 프로그램 수명 동안 유효.
    name: &'static str,
    description: &'static str,
    parameters: Value,
}

impl Tool for McpTool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name,
            description: self.description,
            parameters: self.parameters.clone(),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        self.client.call_tool(&self.mcp_name, args)
    }
}

/// 연결된 MCP 클라이언트의 도구들을 원장 도구로 변환한다.
pub fn tools_from_client(client: Arc<McpClient>) -> Vec<Box<dyn Tool>> {
    let mut out: Vec<Box<dyn Tool>> = Vec::new();
    for def in &client.tools {
        let exposed = sanitize_name(&format!("mcp_{}_{}", client.name, def.name));
        // 프로그램 수명 내내 유효해야 하므로 누수(leak)로 'static 확보(소량).
        let name: &'static str = Box::leak(exposed.into_boxed_str());
        let desc = if def.description.is_empty() {
            format!("[MCP:{}] 외부 도구", client.name)
        } else {
            format!("[MCP:{}] {}", client.name, def.description)
        };
        let description: &'static str = Box::leak(desc.into_boxed_str());
        out.push(Box::new(McpTool {
            client: client.clone(),
            mcp_name: def.name.clone(),
            name,
            description,
            parameters: def.input_schema.clone(),
        }));
    }
    out
}

/// OpenAI 함수 이름 규칙(`[A-Za-z0-9_-]`, 64자 이내)에 맞게 정리.
fn sanitize_name(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.len() > 64 {
        out.truncate(64);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_invalid() {
        assert_eq!(sanitize_name("mcp_fs_read.file"), "mcp_fs_read_file");
        // 비ASCII/특수문자는 모두 '_'로 치환된다.
        assert!(sanitize_name("mcp_깃_커밋")
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    }
}

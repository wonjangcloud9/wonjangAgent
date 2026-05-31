//! 파일시스템 도구: 읽기 / 쓰기 / 디렉터리 목록.

use super::{Tool, ToolContext, ToolSpec};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("'{key}' 인자가 필요합니다"))
}

// ── 파일 읽기 ──────────────────────────────────────────────────────────────

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file",
            description: "파일의 내용을 읽어 반환합니다. 코드나 설정 파일을 확인할 때 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "읽을 파일 경로" }
                },
                "required": ["path"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let path = arg_str(args, "path")?;
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("파일을 읽을 수 없습니다: {path}"))?;
        const MAX: usize = 16000;
        if content.len() > MAX {
            Ok(format!(
                "{}\n… (파일이 길어 {}자에서 잘림)",
                &content[..MAX],
                MAX
            ))
        } else {
            Ok(content)
        }
    }
}

// ── 파일 쓰기 ──────────────────────────────────────────────────────────────

pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_file",
            description:
                "파일에 내용을 씁니다(기존 파일은 덮어씀). 필요한 상위 디렉터리는 자동 생성됩니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "쓸 파일 경로" },
                    "content": { "type": "string", "description": "파일에 쓸 전체 내용" }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let path = arg_str(args, "path")?;
        let content = arg_str(args, "content")?;
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        std::fs::write(path, content).with_context(|| format!("파일을 쓸 수 없습니다: {path}"))?;
        Ok(format!(
            "'{path}'에 {}바이트를 저장했습니다.",
            content.len()
        ))
    }
}

// ── 디렉터리 목록 ──────────────────────────────────────────────────────────

pub struct ListDirTool;

impl Tool for ListDirTool {
    fn name(&self) -> &'static str {
        "list_dir"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_dir",
            description: "디렉터리의 항목 목록을 반환합니다(파일/폴더 구분 포함).",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "목록을 볼 디렉터리 경로(기본: 현재 디렉터리)" }
                }
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let mut entries: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(path)
            .with_context(|| format!("디렉터리를 읽을 수 없습니다: {path}"))?
        {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let kind = if entry.file_type()?.is_dir() {
                "📁"
            } else {
                "📄"
            };
            entries.push(format!("{kind} {name}"));
        }
        entries.sort();
        if entries.is_empty() {
            Ok("(빈 디렉터리)".to_string())
        } else {
            Ok(entries.join("\n"))
        }
    }
}

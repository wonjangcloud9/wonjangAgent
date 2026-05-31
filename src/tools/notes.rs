//! 노트(옵시디언) 도구: 볼트를 검색/읽기/기록한다.

use super::{Tool, ToolContext, ToolSpec};
use crate::config::Config;
use crate::notes;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::PathBuf;

/// 설정에서 볼트 경로를 가져온다(비활성이면 안내 오류).
fn vault_or_err() -> Result<PathBuf> {
    let cfg = Config::load()?;
    notes::vault_path(&cfg.obsidian_vault).ok_or_else(|| {
        anyhow!(
            "옵시디언 볼트가 설정되지 않았습니다. \
             환경 변수 WONJANG_OBSIDIAN_VAULT 또는 설정의 obsidian_vault에 볼트 경로를 지정하세요."
        )
    })
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("'{key}' 인자가 필요합니다"))
}

/// 노트 검색.
pub struct NoteSearchTool;

impl Tool for NoteSearchTool {
    fn name(&self) -> &'static str {
        "note_search"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "note_search",
            description: "옵시디언 볼트의 마크다운 노트에서 키워드를 검색해 일치하는 파일과 \
                줄을 반환합니다. 예전에 적어둔 메모·일지·아이디어를 찾을 때 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "검색어" },
                    "limit": { "type": "integer", "description": "최대 결과 수(기본 20)" }
                },
                "required": ["query"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let vault = vault_or_err()?;
        let query = arg_str(args, "query")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let hits = notes::search(&vault, query, limit)?;
        if hits.is_empty() {
            return Ok(format!("'{query}'에 해당하는 노트를 찾지 못했습니다."));
        }
        let mut out = String::new();
        for h in &hits {
            out.push_str(&format!("{}:{}  {}\n", h.file, h.line_no, h.line));
        }
        Ok(out)
    }
}

/// 노트 읽기.
pub struct NoteReadTool;

impl Tool for NoteReadTool {
    fn name(&self) -> &'static str {
        "note_read"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "note_read",
            description:
                "옵시디언 볼트의 노트 한 개를 읽어 내용을 반환합니다(볼트 기준 상대 경로).",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "볼트 기준 노트 경로(예: '일지/2026-05-31')" }
                },
                "required": ["path"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let vault = vault_or_err()?;
        let path = notes::resolve(&vault, arg_str(args, "path")?)?;
        let content = std::fs::read_to_string(&path)
            .map_err(|_| anyhow!("노트를 읽을 수 없습니다: {}", path.display()))?;
        Ok(content)
    }
}

/// 노트에 기록(덧붙이기).
pub struct NoteAppendTool;

impl Tool for NoteAppendTool {
    fn name(&self) -> &'static str {
        "note_append"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "note_append",
            description: "옵시디언 볼트의 노트에 내용을 덧붙입니다(없으면 생성). 일지 기록, \
                할 일 추가, 메모 캡처 등에 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "볼트 기준 노트 경로(예: '일지/2026-05-31', '인박스')" },
                    "content": { "type": "string", "description": "덧붙일 내용(마크다운)" }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let vault = vault_or_err()?;
        let path = arg_str(args, "path")?;
        let content = arg_str(args, "content")?;
        let written = notes::append(&vault, path, content)?;
        Ok(format!("'{}'에 기록했습니다.", written.display()))
    }
}

/// 노트 목록.
pub struct NoteListTool;

impl Tool for NoteListTool {
    fn name(&self) -> &'static str {
        "note_list"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "note_list",
            description: "옵시디언 볼트의 마크다운 노트 목록(볼트 기준 상대 경로)을 반환합니다.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let vault = vault_or_err()?;
        let files = notes::list_markdown(&vault)?;
        if files.is_empty() {
            return Ok("(볼트에 노트가 없습니다)".to_string());
        }
        Ok(files.join("\n"))
    }
}

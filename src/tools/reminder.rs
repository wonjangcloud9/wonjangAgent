//! 알림 도구: 약속/할 일을 시각과 함께 등록·조회·삭제한다.

use super::{Tool, ToolContext, ToolSpec};
use crate::reminders::{self, ReminderStore};
use anyhow::{anyhow, Result};
use serde_json::Value;

/// 알림 추가.
pub struct AddReminderTool;

impl Tool for AddReminderTool {
    fn name(&self) -> &'static str {
        "add_reminder"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "add_reminder",
            description: "약속·할 일을 시각과 함께 등록합니다. 상대 시간은 in_minutes(지금부터 N분 \
                뒤), 절대 시간은 at_unix(epoch 초)로 주세요. 절대 시각은 셸 date 명령으로 epoch를 \
                계산할 수 있습니다(예: macOS `date -j -f '%Y-%m-%d %H:%M' '2026-06-01 15:00' +%s`). \
                크론 데몬(wonjang cron run)이 켜져 있으면 때가 됐을 때 데스크탑 알림을 띄웁니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "알림 내용(예: '치과 예약')" },
                    "in_minutes": { "type": "integer", "description": "지금부터 N분 뒤(상대 시간)" },
                    "at_unix": { "type": "integer", "description": "알릴 절대 시각(epoch 초)" }
                },
                "required": ["title"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'title' 인자가 필요합니다"))?;
        let now = reminders::now_unix();
        let at = if let Some(m) = args.get("in_minutes").and_then(|v| v.as_i64()) {
            now + m * 60
        } else if let Some(t) = args.get("at_unix").and_then(|v| v.as_i64()) {
            t
        } else {
            return Err(anyhow!("'in_minutes' 또는 'at_unix' 중 하나가 필요합니다"));
        };
        let mut store = ReminderStore::load()?;
        let id = store.add(at, title)?;
        Ok(format!(
            "알림 #{id} 등록: '{title}' ({})",
            reminders::relative(at, now)
        ))
    }
}

/// 알림 목록.
pub struct ListRemindersTool;

impl Tool for ListRemindersTool {
    fn name(&self) -> &'static str {
        "list_reminders"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_reminders",
            description: "예정된 약속·할 일 목록을 시각순으로 반환합니다.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let store = ReminderStore::load()?;
        let now = reminders::now_unix();
        let up = store.upcoming(now);
        if up.is_empty() {
            return Ok("예정된 알림이 없습니다.".to_string());
        }
        let mut out = String::new();
        for r in up {
            out.push_str(&format!(
                "#{}  {} — {}\n",
                r.id,
                r.title,
                reminders::relative(r.at_unix, now)
            ));
        }
        Ok(out)
    }
}

/// 알림 삭제.
pub struct RemoveReminderTool;

impl Tool for RemoveReminderTool {
    fn name(&self) -> &'static str {
        "remove_reminder"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "remove_reminder",
            description: "id로 예정된 알림을 삭제합니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": "삭제할 알림 id" } },
                "required": ["id"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("'id' 인자가 필요합니다"))?;
        let mut store = ReminderStore::load()?;
        if store.remove(id)? {
            Ok(format!("알림 #{id}을(를) 삭제했습니다."))
        } else {
            Ok(format!("알림 #{id}을(를) 찾을 수 없습니다."))
        }
    }
}

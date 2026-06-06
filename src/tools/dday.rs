//! 디데이 도구: 중요한 날을 등록/조회한다.

use super::{Tool, ToolContext, ToolSpec};
use crate::ddays::{self, DdayStore};
use anyhow::{anyhow, Result};
use serde_json::Value;

pub struct AddDdayTool;

impl Tool for AddDdayTool {
    fn name(&self) -> &'static str {
        "add_dday"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "add_dday",
            description: "중요한 날(디데이)을 등록합니다. 수능, 기념일, 마감일 등 남은 날짜를 \
                세고 싶은 날에 사용하세요. 날짜는 YYYY-MM-DD 형식.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "label": { "type": "string", "description": "디데이 이름(예: '수능')" },
                    "date": { "type": "string", "description": "목표 날짜 YYYY-MM-DD" }
                },
                "required": ["label", "date"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let label = args
            .get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'label' 인자가 필요합니다"))?;
        let date = args
            .get("date")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'date' 인자가 필요합니다"))?;
        let annual = args
            .get("annual")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mut store = DdayStore::load()?;
        let id = store.add(label, date, annual)?;
        let today = ddays::today();
        let eff = ddays::effective_date(ddays::parse_date(date)?, annual, today);
        Ok(format!(
            "디데이 #{id} 등록: {label}{} ({}, {})",
            if annual { " 🔁매년" } else { "" },
            eff.format("%Y-%m-%d"),
            ddays::dday_label(ddays::days_until(eff, today))
        ))
    }
}

pub struct ListDdaysTool;

impl Tool for ListDdaysTool {
    fn name(&self) -> &'static str {
        "list_ddays"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_ddays",
            description: "등록된 디데이 목록을 남은 날짜와 함께 반환합니다.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let store = DdayStore::load()?;
        if store.all().is_empty() {
            return Ok("등록된 디데이가 없습니다.".to_string());
        }
        let today = ddays::today();
        let mut out = String::new();
        for d in store.all() {
            let (label, datestr) = match ddays::parse_date(&d.date) {
                Ok(dt) => {
                    let eff = ddays::effective_date(dt, d.annual, today);
                    (
                        ddays::dday_label(ddays::days_until(eff, today)),
                        eff.format("%Y-%m-%d").to_string(),
                    )
                }
                Err(_) => ("?".to_string(), d.date.clone()),
            };
            let tag = if d.annual { " 🔁" } else { "" };
            out.push_str(&format!(
                "#{}  {}  {}{tag} ({datestr})\n",
                d.id, label, d.label
            ));
        }
        Ok(out)
    }
}

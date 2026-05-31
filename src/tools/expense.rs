//! 가계부 도구: 지출 기록/요약.

use super::{Tool, ToolContext, ToolSpec};
use crate::expenses::{self, ExpenseStore};
use anyhow::{anyhow, Result};
use serde_json::Value;

pub struct AddExpenseTool;

impl Tool for AddExpenseTool {
    fn name(&self) -> &'static str {
        "add_expense"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "add_expense",
            description:
                "오늘 지출을 가계부에 기록합니다. 금액(원), 분류(식비/교통/배달/카페 등), \
                선택 메모를 받습니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "amount": { "type": "integer", "description": "금액(원)" },
                    "category": { "type": "string", "description": "분류(예: 식비, 교통, 배달)" },
                    "note": { "type": "string", "description": "메모(선택)" }
                },
                "required": ["amount", "category"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let amount = args
            .get("amount")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow!("'amount'(정수)가 필요합니다"))?;
        let category = args
            .get("category")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'category' 인자가 필요합니다"))?;
        let note = args.get("note").and_then(|v| v.as_str()).unwrap_or("");
        let mut store = ExpenseStore::load()?;
        let id = store.add(amount, category, note)?;
        let today = expenses::today_str();
        Ok(format!(
            "지출 #{id} 기록: {} ({category}). 오늘 합계 {}",
            expenses::won(amount),
            expenses::won(store.total_on(&today))
        ))
    }
}

pub struct ExpenseSummaryTool;

impl Tool for ExpenseSummaryTool {
    fn name(&self) -> &'static str {
        "expense_summary"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "expense_summary",
            description: "오늘과 이번 달 지출 합계, 이번 달 분류별 지출을 반환합니다.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    fn execute(&self, _args: &Value, _ctx: &ToolContext) -> Result<String> {
        let store = ExpenseStore::load()?;
        let today = expenses::today_str();
        let ym = expenses::this_month();
        let mut out = format!(
            "오늘({today}) 지출: {}\n이번 달({ym}) 지출: {}\n",
            expenses::won(store.total_on(&today)),
            expenses::won(store.total_in_month(&ym))
        );
        let by = store.by_category_in_month(&ym);
        if !by.is_empty() {
            out.push_str("분류별:\n");
            for (cat, amt) in by {
                out.push_str(&format!("  {cat}: {}\n", expenses::won(amt)));
            }
        }
        Ok(out)
    }
}

//! 환율 도구: 실시간 환율/환산.

use super::{Tool, ToolContext, ToolSpec};
use crate::exchange;
use anyhow::Result;
use serde_json::Value;

pub struct ExchangeTool;

impl Tool for ExchangeTool {
    fn name(&self) -> &'static str {
        "exchange_rate"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "exchange_rate",
            description: "실시간 환율을 가져와 원화로 환산합니다. 통화(예: USD, JPY)와 선택 \
                금액을 받습니다. 통화를 생략하면 USD·JPY·EUR·CNY의 원화 환율을 보여줍니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "currency": { "type": "string", "description": "통화 코드(USD/JPY/EUR/CNY 등)" },
                    "amount": { "type": "number", "description": "환산할 금액(선택)" }
                }
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let currency = args
            .get("currency")
            .and_then(|v| v.as_str())
            .map(String::from);
        let amount = args.get("amount").and_then(|v| v.as_f64());
        let (_, rates) = crate::util::run_async(async move { exchange::fetch().await })?;

        match currency {
            Some(cur) => {
                let per = exchange::krw_per(&cur, &rates)
                    .ok_or_else(|| anyhow::anyhow!("'{cur}' 환율을 찾을 수 없습니다"))?;
                let amt = amount.unwrap_or(1.0);
                Ok(format!(
                    "{} {} = {}원",
                    exchange::comma(amt, 0),
                    cur.to_uppercase(),
                    exchange::comma(amt * per, 0)
                ))
            }
            None => {
                let mut out = String::new();
                for (code, unit) in [("USD", 1.0), ("JPY", 100.0), ("EUR", 1.0), ("CNY", 1.0)] {
                    if let Some(per) = exchange::krw_per(code, &rates) {
                        out.push_str(&format!(
                            "{} {code} = {}원\n",
                            exchange::comma(unit, 0),
                            exchange::comma(unit * per, 0)
                        ));
                    }
                }
                Ok(out)
            }
        }
    }
}

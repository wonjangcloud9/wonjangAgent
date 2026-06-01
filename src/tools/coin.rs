//! 코인 도구: 업비트 실시간 시세.

use super::{Tool, ToolContext, ToolSpec};
use crate::{coin, exchange};
use anyhow::Result;
use serde_json::Value;

pub struct CoinTool;

impl Tool for CoinTool {
    fn name(&self) -> &'static str {
        "coin_price"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "coin_price",
            description: "업비트(KRW 마켓)의 실시간 코인 시세와 전일 대비 변동률을 반환합니다. \
                심볼(예: BTC, ETH)을 주거나 생략하면 인기 코인들을 보여줍니다.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "코인 심볼(예: BTC). 생략 시 인기 코인" }
                }
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let markets = match args.get("symbol").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => {
                vec![format!("KRW-{}", s.trim().to_uppercase())]
            }
            _ => coin::default_markets(),
        };
        let coins = crate::util::run_async(async move { coin::fetch(&markets).await })?;
        let mut out = String::new();
        for c in &coins {
            out.push_str(&format!(
                "{} {}원 ({:+.2}%)\n",
                c.symbol,
                exchange::comma(c.price, 0),
                c.change_pct
            ));
        }
        Ok(out)
    }
}

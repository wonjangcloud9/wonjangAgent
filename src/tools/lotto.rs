//! 로또 도구: 자동 번호 추첨.

use super::{Tool, ToolContext, ToolSpec};
use crate::lotto;
use anyhow::Result;
use serde_json::Value;

pub struct LottoTool;

impl Tool for LottoTool {
    fn name(&self) -> &'static str {
        "lotto_numbers"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "lotto_numbers",
            description: "로또 자동 번호(1~45 중 6개)를 생성합니다. games로 게임 수를 지정할 수 \
                있습니다(기본 5). 재미로 쓰세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "games": { "type": "integer", "description": "생성할 게임 수(기본 5)" }
                }
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let games = args
            .get("games")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .clamp(1, 10) as usize;
        let mut out = String::new();
        for (i, g) in lotto::auto(games).iter().enumerate() {
            let label = (b'A' + i as u8) as char;
            let nums: Vec<String> = g.iter().map(|n| n.to_string()).collect();
            out.push_str(&format!("{label}  {}\n", nums.join(", ")));
        }
        Ok(out)
    }
}

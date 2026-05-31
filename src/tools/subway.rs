//! 지하철 도구: 서울 지하철 실시간 도착정보.

use super::{Tool, ToolContext, ToolSpec};
use crate::{config::Config, subway};
use anyhow::{anyhow, Result};
use serde_json::Value;

pub struct SubwayTool;

impl Tool for SubwayTool {
    fn name(&self) -> &'static str {
        "subway_arrivals"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "subway_arrivals",
            description: "서울 지하철 역의 실시간 도착정보를 가져옵니다(역 이름만 주면 됨). \
                '지하철 언제 와', '강남역 도착정보' 같은 요청에 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "station": { "type": "string", "description": "역 이름(예: 강남, 서울, 홍대입구)" }
                },
                "required": ["station"]
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let station = args
            .get("station")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'station' 인자가 필요합니다"))?
            .to_string();
        let key = Config::load()?.seoul_api_key;
        let list =
            crate::util::run_async(async move { subway::arrivals(&key, &station, 10).await })?;
        if list.is_empty() {
            return Ok("도착 정보가 없습니다(역 이름을 확인하세요).".to_string());
        }
        let mut out = String::new();
        for a in &list {
            out.push_str(&format!("[{}] {} — {}\n", a.line, a.direction, a.message));
        }
        Ok(out)
    }
}

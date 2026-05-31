//! 날씨 도구: open-meteo 실시간 날씨.

use super::{Tool, ToolContext, ToolSpec};
use crate::weather;
use anyhow::Result;
use serde_json::Value;

pub struct WeatherTool;

impl Tool for WeatherTool {
    fn name(&self) -> &'static str {
        "weather_now"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "weather_now",
            description: "지역의 실시간 날씨를 가져옵니다(기온·체감·습도·강수·최저최고). \
                지역을 생략하면 서울. 날씨 질문이나 브리핑에 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string", "description": "지역 이름(예: 서울, 부산). 생략 시 서울" }
                }
            }),
        }
    }

    fn execute(&self, args: &Value, _ctx: &ToolContext) -> Result<String> {
        let loc = args
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let w = crate::util::run_async(async move { weather::weather(&loc).await })?;
        Ok(format!(
            "{} 날씨: {} {:.0}°C (체감 {:.0}°C), 습도 {}%, 강수 {}mm, 오늘 {:.0}~{:.0}°C",
            w.place, w.desc, w.temp, w.feels, w.humidity, w.precip, w.today_min, w.today_max
        ))
    }
}

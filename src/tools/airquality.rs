//! 대기질 도구: 미세먼지(PM10/PM2.5).

use super::{Tool, ToolContext, ToolSpec};
use crate::airquality;
use anyhow::Result;
use serde_json::Value;

pub struct AirQualityTool;

impl Tool for AirQualityTool {
    fn name(&self) -> &'static str {
        "air_quality"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "air_quality",
            description: "지역의 미세먼지(PM10)·초미세먼지(PM2.5)와 환경부 기준 등급을 \
                반환합니다. 지역을 생략하면 서울. '미세먼지 어때' 같은 요청에 사용하세요.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string", "description": "지역 이름(생략 시 서울)" }
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
        let a = crate::util::run_async(async move { airquality::air_quality(&loc).await })?;
        let (g25, e25) = airquality::grade_pm25(a.pm25);
        let (g10, e10) = airquality::grade_pm10(a.pm10);
        Ok(format!(
            "{} 미세먼지: PM10 {:.0}({g10} {e10}), 초미세먼지 PM2.5 {:.0}({g25} {e25})",
            a.place, a.pm10, a.pm25
        ))
    }
}

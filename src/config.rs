//! 설정 로딩.
//!
//! 우선순위: 환경 변수 > 설정 파일(`~/.config/wonjang/config.toml`) > 기본값.
//! 헤르메스 에이전트처럼 제공자(provider) 무관 설계를 위해 OpenAI 호환
//! 엔드포인트를 기본으로 한다(OpenRouter, OpenAI, DeepSeek, 로컬 vLLM 등).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// OpenAI 호환 채팅 완성 엔드포인트의 베이스 URL.
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// 사용할 모델 이름.
    #[serde(default = "default_model")]
    pub model: String,

    /// API 키. 비어 있으면 환경 변수에서만 읽는다(설정 파일에 평문 저장 회피).
    #[serde(default)]
    pub api_key: String,

    /// 한 작업당 최대 에이전트 루프(도구 호출 왕복) 횟수.
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,
}

fn default_base_url() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

fn default_model() -> String {
    "anthropic/claude-3.5-sonnet".to_string()
}

fn default_max_steps() -> u32 {
    25
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            model: default_model(),
            api_key: String::new(),
            max_steps: default_max_steps(),
        }
    }
}

/// 설정 파일 경로(`~/.config/wonjang/config.toml`).
pub fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir().context("설정 디렉터리를 찾을 수 없습니다")?;
    Ok(dir.join("wonjang").join("config.toml"))
}

impl Config {
    /// 파일 + 환경 변수를 병합해 설정을 로드한다.
    pub fn load() -> Result<Self> {
        let mut cfg = match config_path() {
            Ok(path) if path.exists() => {
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("설정 파일을 읽을 수 없습니다: {}", path.display()))?;
                toml::from_str(&text).context("설정 파일(TOML) 형식이 올바르지 않습니다")?
            }
            _ => Config::default(),
        };

        // 환경 변수로 덮어쓰기.
        if let Ok(v) = std::env::var("WONJANG_BASE_URL") {
            if !v.is_empty() {
                cfg.base_url = v;
            }
        }
        if let Ok(v) = std::env::var("WONJANG_MODEL") {
            if !v.is_empty() {
                cfg.model = v;
            }
        }
        // API 키: 전용 변수 우선, 없으면 흔한 변수로 폴백.
        for key in ["WONJANG_API_KEY", "OPENROUTER_API_KEY", "OPENAI_API_KEY"] {
            if let Ok(v) = std::env::var(key) {
                if !v.is_empty() {
                    cfg.api_key = v;
                    break;
                }
            }
        }

        Ok(cfg)
    }

    /// 설정 파일을 디스크에 기록한다(API 키는 저장하지 않는다).
    pub fn save(&self) -> Result<PathBuf> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut to_save = self.clone();
        to_save.api_key = String::new(); // 보안: 키는 파일에 남기지 않는다.
        let text = toml::to_string_pretty(&to_save)?;
        std::fs::write(&path, text)?;
        Ok(path)
    }
}

//! 페르소나(성격) — "내 비서"라는 느낌을 만드는 핵심.
//!
//! 헤르메스 에이전트의 SOUL.md처럼, 에이전트의 말투·태도를 **사용자가 소유하는
//! 파일**(`~/.local/share/wonjang/SOUL.md`)로 빼낸다. 이 파일이 있으면 시스템
//! 프롬프트 맨 앞에 주입되어 기본 성격을 대체한다. `성격` 명령으로 프리셋을
//! 고르거나 파일을 직접 편집해 나만의 비서를 만든다.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

/// 성격 프리셋 (키, 한글이름, 페르소나 본문).
pub const PRESETS: &[(&str, &str, &str)] = &[
    (
        "기본",
        "다정한 비서",
        "당신은 '원장'입니다. 사용자의 든든하고 다정한 한국어 개인비서예요. \
         따뜻하지만 프로페셔널하고, 사용자를 잘 알고 먼저 챙기려 합니다. \
         존댓말을 쓰되 딱딱하지 않게, 가끔 가벼운 이모지로 친근함을 더합니다.",
    ),
    (
        "친구",
        "편한 친구",
        "당신은 '원장'입니다. 사용자의 오래된 친구처럼 **반말**로 편하게 말합니다. \
         장난기 있고 솔직하지만, 정작 도와줄 땐 확실하게 챙겨줘요. \
         '야', '그거 내가 해줄게', '오케이' 같은 편한 말투를 씁니다.",
    ),
    (
        "집사",
        "정중한 집사",
        "당신은 '원장'입니다. 충실하고 격식 있는 집사예요. 사용자를 '주인님'이라 \
         부르고 정중한 존댓말을 씁니다. 차분하고 품위 있게, 빈틈없이 모십니다.",
    ),
    (
        "선배",
        "시크한 선배",
        "당신은 '원장'입니다. 무뚝뚝하고 시크하지만 은근히 잘 챙겨주는 선배예요. \
         말은 짧고 군더더기 없이. 칭찬은 인색해도 할 일은 확실히 해줍니다.",
    ),
    (
        "발랄",
        "밝은 에너지",
        "당신은 '원장'입니다. 밝고 에너지 넘치는 비서예요! 칭찬을 잘하고 응원을 \
         아끼지 않습니다. 이모지를 적극 쓰고, 사용자가 힘이 나게 격려합니다. ✨",
    ),
];

/// 기본 페르소나(SOUL.md가 없을 때).
pub fn default_persona() -> &'static str {
    PRESETS[0].2
}

/// 프리셋 키로 페르소나 본문을 찾는다.
pub fn preset(key: &str) -> Option<&'static str> {
    let k = key.trim();
    PRESETS
        .iter()
        .find(|(name, _, _)| *name == k)
        .map(|(_, _, body)| *body)
}

/// SOUL.md 경로.
pub fn soul_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("SOUL.md"))
}

/// 현재 활성 페르소나(SOUL.md가 있으면 그 내용, 없으면 기본).
pub fn active_persona() -> String {
    match soul_path().ok().filter(|p| p.exists()) {
        Some(p) => std::fs::read_to_string(&p)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default_persona().to_string()),
        None => default_persona().to_string(),
    }
}

/// 프리셋을 SOUL.md에 저장한다.
pub fn set_preset(key: &str) -> Result<()> {
    let body = preset(key).ok_or_else(|| {
        let names: Vec<&str> = PRESETS.iter().map(|(n, _, _)| *n).collect();
        anyhow!("'{key}' 성격은 없어요. 가능: {}", names.join(" / "))
    })?;
    std::fs::write(soul_path()?, body)?;
    Ok(())
}

/// SOUL.md를 지워 기본 성격으로 되돌린다.
pub fn reset() -> Result<()> {
    let p = soul_path()?;
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_lookup() {
        assert!(preset("친구").unwrap().contains("반말"));
        assert!(preset("집사").unwrap().contains("주인님"));
        assert!(preset("없음").is_none());
    }

    #[test]
    fn default_is_first_preset() {
        assert_eq!(default_persona(), PRESETS[0].2);
        assert!(default_persona().contains("원장"));
    }
}

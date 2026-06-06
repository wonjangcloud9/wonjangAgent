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
    // 별칭: README·온보딩이 기본 페르소나를 '다정'(다정한 비서)이라 부르므로 받아들인다.
    let k = if k == "다정" { "기본" } else { k };
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

/// 현재 활성 성격의 프리셋 키(직접 편집해 매칭 안 되면 "나만의").
pub fn active_preset_key() -> &'static str {
    let active = active_persona();
    PRESETS
        .iter()
        .find(|(_, _, body)| active == *body)
        .map(|(k, _, _)| *k)
        .unwrap_or("나만의")
}

/// 성격·시간대에 맞춘 짧은 첫 인사(배너용).
pub fn greeting() -> String {
    use chrono::Timelike;
    let hour = chrono::Local::now().hour();
    let tod = match hour {
        5..=10 => "좋은 아침이에요",
        11..=16 => "안녕하세요",
        17..=20 => "좋은 저녁이에요",
        _ => "안녕하세요",
    };
    match active_preset_key() {
        "친구" => "안녕! 나 원장이야 🙌".to_string(),
        "집사" => "주인님, 원장 대령했습니다.".to_string(),
        "선배" => "왔냐. 원장이다.".to_string(),
        "발랄" => format!("{tod}! 원장이에요! ✨"),
        _ => format!("{tod}, 원장입니다 🌙"),
    }
}

/// 프리셋별 짧은 얼굴(이모지) — 터미널에서 '캐릭터가 곁에 있다'는 느낌을 준다.
pub fn face(key: &str) -> &'static str {
    match key {
        "친구" => "😎",
        "집사" => "🎩",
        "선배" => "😏",
        "발랄" => "✨",
        "나만의" => "🌟",
        _ => "🌙",
    }
}

/// 사용자가 한 줄로 묘사한 캐릭터를 페르소나 본문으로 조립한다(직접 만들기).
/// 키·LLM 없이 결정론적으로 만들어, SOUL.md에 저장하면 '나만의' 원장이 된다.
pub fn build_custom_persona(desc: &str, formal: bool) -> String {
    let tone = if formal {
        "존댓말로 정중하되 딱딱하지 않게"
    } else {
        "반말로 편하게"
    };
    format!(
        "당신은 '원장'입니다. {}. 항상 한국어로, {} 말하며 사용자를 든든하게 먼저 챙깁니다.",
        desc.trim().trim_end_matches(['.', '。']),
        tone
    )
}

/// 임의의 페르소나 본문을 SOUL.md에 저장한다(직접 만들기·외부 편집 결과 저장용).
pub fn save_persona(body: &str) -> Result<()> {
    std::fs::write(soul_path()?, body.trim())?;
    Ok(())
}

/// 프리셋별 자기소개 한마디 — 고를 때 미리듣기 + 고른 뒤 첫 인사로 쓴다.
pub fn voice_sample(key: &str) -> &'static str {
    match key {
        "친구" => "야, 나 원장이야. 뭐 도와줄까? 😎",
        "집사" => "주인님, 원장 대령했습니다. 분부만 내려주십시오.",
        "선배" => "왔냐. 뭐 필요한데.",
        "발랄" => "안녕하세요!! 원장이에요!! 오늘도 화이팅이에요 ✨",
        _ => "안녕하세요, 원장입니다 🌙 무엇을 도와드릴까요?",
    }
}

/// 사용자가 성격을 직접 고른 적이 있는가(있으면 첫 실행 선택을 다시 묻지 않는다).
fn chosen_marker() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("wonjang").join(".persona_chosen"))
}

/// 성격을 한 번이라도 정했으면 true(마커 또는 SOUL.md 존재).
pub fn is_chosen() -> bool {
    chosen_marker().map(|p| p.exists()).unwrap_or(false)
        || soul_path().map(|p| p.exists()).unwrap_or(false)
}

/// 성격을 골랐다고 표시(첫 실행 선택을 반복하지 않도록).
pub fn mark_chosen() {
    if let Some(p) = chosen_marker() {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(p, "1").ok();
    }
}

/// 명령을 실행하기 직전 원장이 자기 말투로 던지는 한마디(성격별). `seed`(입력 텍스트)로
/// 결정론적으로 한 줄 고른다 — 같은 입력엔 같은 반응, 입력마다 다양하게. 키·LLM 불필요.
pub fn ack(key: &str, seed: &str) -> &'static str {
    let lines: &[&str] = match key {
        "친구" => &[
            "오케이, 바로 해줄게 👌",
            "그거 내가 해줄게",
            "야 잠깐, 본다",
            "ㅇㅋ 가자 🙌",
        ],
        "집사" => &[
            "분부대로 하겠습니다.",
            "바로 처리하겠습니다, 주인님.",
            "잠시만 기다려 주십시오.",
            "명 받들겠습니다.",
        ],
        "선배" => &["어, 해줄게.", "본다.", "잠깐.", "그래, 보자."],
        "발랄" => &[
            "네!! 바로 갈게요~ ✨",
            "맡겨주세요! 💪",
            "오케이! 🙌",
            "좋아요! 바로요! 🌟",
        ],
        _ => &[
            "네, 바로 볼게요 😊",
            "잠깐만요, 처리할게요",
            "네, 도와드릴게요",
            "확인할게요!",
        ],
    };
    let idx = seed.bytes().map(|b| b as usize).sum::<usize>() % lines.len();
    lines[idx]
}

/// 선택 메뉴 입력("" 또는 "1".."N")을 프리셋 인덱스로. 빈 값·범위 밖·숫자 아님은 기본(0).
pub fn resolve_choice(input: &str) -> usize {
    let t = input.trim();
    if t.is_empty() {
        return 0;
    }
    t.parse::<usize>()
        .ok()
        .filter(|n| (1..=PRESETS.len()).contains(n))
        .map(|n| n - 1)
        .unwrap_or(0)
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
    fn resolve_choice_maps_menu_input() {
        assert_eq!(resolve_choice(""), 0); // 엔터 = 기본(첫 번째)
        assert_eq!(resolve_choice("1"), 0);
        assert_eq!(resolve_choice("2"), 1);
        assert_eq!(
            resolve_choice(&PRESETS.len().to_string()),
            PRESETS.len() - 1
        );
        assert_eq!(resolve_choice("99"), 0); // 범위 밖 = 기본
        assert_eq!(resolve_choice("abc"), 0); // 숫자 아님 = 기본
        assert_eq!(resolve_choice("  3  "), 2); // 공백 허용
    }

    #[test]
    fn build_custom_persona_includes_desc_and_tone() {
        let formal = build_custom_persona("나를 '대표님'이라 부르는 비서", true);
        assert!(formal.contains("원장"));
        assert!(formal.contains("대표님"));
        assert!(formal.contains("존댓말"));
        let casual = build_custom_persona("나를 형이라 부르는 차분한 친구.", false);
        assert!(casual.contains("반말"));
        // 끝의 마침표는 정리된다(중복 방지).
        assert!(!casual.contains("친구.."));
    }

    #[test]
    fn ack_is_deterministic_and_per_persona() {
        // 같은 입력 → 같은 반응(결정론).
        assert_eq!(ack("친구", "연봉 3600"), ack("친구", "연봉 3600"));
        // 모든 프리셋이 비지 않은 반응.
        for (k, _, _) in PRESETS {
            assert!(!ack(k, "자랑").is_empty());
        }
        // 성격마다 말투가 다르다(라인 집합이 겹치지 않음).
        assert_ne!(ack("친구", "자랑"), ack("집사", "자랑"));
        assert_ne!(ack("선배", "자랑"), ack("발랄", "자랑"));
    }

    #[test]
    fn every_preset_has_distinct_face_and_voice() {
        for (key, _, _) in PRESETS {
            assert!(!face(key).is_empty());
            assert!(!voice_sample(key).is_empty());
        }
        // 프리셋마다 목소리가 달라야 개성이 산다.
        let voices: std::collections::HashSet<_> =
            PRESETS.iter().map(|(k, _, _)| voice_sample(k)).collect();
        assert_eq!(voices.len(), PRESETS.len());
    }

    #[test]
    fn preset_lookup() {
        assert!(preset("친구").unwrap().contains("반말"));
        assert!(preset("집사").unwrap().contains("주인님"));
        assert!(preset("없음").is_none());
    }

    #[test]
    fn dajeong_alias_maps_to_default() {
        // README·온보딩이 광고하는 '다정'이 기본(다정한 비서) 페르소나로 선택돼야 한다.
        assert_eq!(preset("다정"), preset("기본"));
        assert_eq!(preset("다정"), Some(default_persona()));
    }

    #[test]
    fn default_is_first_preset() {
        assert_eq!(default_persona(), PRESETS[0].2);
        assert!(default_persona().contains("원장"));
    }
}

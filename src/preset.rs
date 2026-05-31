//! 작업 프리셋 — 한국 사용자 편의 기능.
//!
//! 자주 쓰는 로컬 작업을 한국어 이름의 프리셋으로 묶어, 한 번의 명령으로
//! 실행한다. 빌트인 프리셋에 더해 사용자가 직접
//! `~/.config/wonjang/presets.toml`에 자신만의 프리셋을 추가할 수 있다.

use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

/// 하나의 작업 프리셋.
#[derive(Debug, Clone, Deserialize)]
pub struct Preset {
    /// 대표 이름(한국어).
    pub name: String,
    /// 한 줄 설명.
    pub description: String,
    /// 에이전트에게 전달할 요청문.
    pub prompt: String,
    /// 별칭(영문/축약 등).
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl Preset {
    fn new(name: &str, description: &str, aliases: &[&str], prompt: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            prompt: prompt.to_string(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// 이름이나 별칭이 주어진 키와 일치하는가.
    fn matches(&self, key: &str) -> bool {
        self.name == key || self.aliases.iter().any(|a| a == key)
    }
}

/// 빌트인 프리셋 모음(한국 데스크탑 환경 중심).
pub fn builtin() -> Vec<Preset> {
    vec![
        Preset::new(
            "다운로드정리",
            "다운로드 폴더를 종류별 하위 폴더로 정리",
            &["downloads", "정리"],
            "사용자의 다운로드 폴더(~/Downloads)를 list_dir로 살펴보고, 파일을 종류별\
             (이미지/문서/압축/동영상/오디오/기타)로 하위 폴더를 만들어 '이동'해서 정리해줘. \
             먼저 어떤 파일이 있는지 확인하고 이동 계획을 한국어로 보여준 다음 진행해. \
             원본을 삭제하지 말고 이동만 하고, 끝나면 무엇을 어디로 옮겼는지 요약해줘.",
        ),
        Preset::new(
            "바탕화면정리",
            "바탕화면의 스크린샷/파일을 정리",
            &["desktop", "데탑정리"],
            "바탕화면(~/Desktop)을 살펴보고 파일을 정리해줘. 스크린샷(파일명에 \
             'Screenshot', '스크린샷', 'CleanShot' 등이 들어간 이미지)은 'Screenshots' 폴더로 \
             모으고, 나머지는 종류별로 정리해. 진행 전 계획을 보여주고, 이동만(삭제 금지) 해줘.",
        ),
        Preset::new(
            "큰파일",
            "현재 폴더에서 용량 큰 파일/폴더 찾기",
            &["bigfiles", "용량"],
            "현재 작업 디렉터리에서 용량이 큰 파일과 폴더 상위 15개를, 사람이 읽기 쉬운 \
             크기 단위로 정렬해서 표로 보여줘. du, sort 등 적절한 명령을 사용해.",
        ),
        Preset::new(
            "오늘커밋",
            "오늘 한 git 커밋 요약",
            &["today", "커밋요약"],
            "현재 git 저장소에서 오늘 날짜에 작성된 커밋들을 찾아, 무슨 작업을 했는지 \
             한국어로 항목별로 요약해줘. git log를 활용하고, git 저장소가 아니면 그 사실을 알려줘.",
        ),
        Preset::new(
            "포트",
            "열려 있는 포트와 프로세스 확인",
            &["ports", "포트확인"],
            "지금 이 컴퓨터에서 LISTEN 상태인 네트워크 포트와 각 포트를 점유한 프로세스를 \
             확인해서 표로 보여줘(lsof -i -P -n 또는 netstat 활용). 포트번호, 프로세스명, PID를 포함해.",
        ),
        Preset::new(
            "와이파이",
            "현재 와이파이/네트워크 정보",
            &["wifi", "네트워크"],
            "현재 연결된 와이파이 이름(SSID)과 로컬 IP 주소, 공인 IP를 확인해서 알려줘. \
             운영체제에 맞는 명령을 사용하고, 공인 IP는 web_fetch로 확인해도 좋아.",
        ),
        Preset::new(
            "정돈",
            "현재 폴더의 어수선한 파일 정리 제안",
            &["tidy"],
            "현재 작업 디렉터리를 list_dir로 살펴보고, 어수선하게 흩어진 파일이 있으면 \
             어떻게 정리하면 좋을지 한국어로 제안해줘. 실제 이동은 사용자 동의를 받은 뒤에만 해.",
        ),
    ]
}

/// 사용자 프리셋 파일 경로.
pub fn user_presets_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("설정 디렉터리를 찾을 수 없습니다"))?
        .join("wonjang");
    Ok(dir.join("presets.toml"))
}

#[derive(Deserialize)]
struct UserPresetsFile {
    #[serde(default)]
    preset: Vec<Preset>,
}

/// 사용자 프리셋을 로드(없거나 오류면 빈 목록).
fn user_presets() -> Vec<Preset> {
    let path = match user_presets_path() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    toml::from_str::<UserPresetsFile>(&text)
        .map(|f| f.preset)
        .unwrap_or_default()
}

/// 빌트인 + 사용자 프리셋 전체(같은 이름은 사용자 것이 우선).
pub fn load_all() -> Vec<Preset> {
    let mut all = builtin();
    for up in user_presets() {
        if let Some(existing) = all.iter_mut().find(|p| p.name == up.name) {
            *existing = up;
        } else {
            all.push(up);
        }
    }
    all
}

/// 이름/별칭으로 프리셋을 찾는다.
pub fn find(key: &str) -> Option<Preset> {
    load_all().into_iter().find(|p| p.matches(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_have_unique_names() {
        let presets = builtin();
        let mut names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "빌트인 프리셋 이름이 중복됨");
    }

    #[test]
    fn find_by_name_and_alias() {
        assert!(find("다운로드정리").is_some());
        assert!(find("정리").is_some()); // 별칭
        assert!(find("downloads").is_some()); // 별칭
        assert!(find("없는프리셋").is_none());
    }

    #[test]
    fn matches_logic() {
        let p = Preset::new("테스트", "d", &["t", "test"], "p");
        assert!(p.matches("테스트"));
        assert!(p.matches("test"));
        assert!(!p.matches("다른거"));
    }
}

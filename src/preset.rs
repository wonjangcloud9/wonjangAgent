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
        Preset::new(
            "디스크",
            "디스크 용량 현황 확인",
            &["disk", "용량현황"],
            "이 컴퓨터의 디스크 용량 현황(전체/사용/남은 용량)을 df -h로 확인해서 사람이 \
             읽기 쉽게 한국어로 정리해줘.",
        ),
        Preset::new(
            "중복파일",
            "현재 폴더의 중복 파일 찾기",
            &["dups", "중복"],
            "현재 작업 디렉터리(하위 포함)에서 내용이 같은 중복 파일을 찾아줘. 파일 크기로 \
             1차 후보를 추린 뒤 해시(shasum/md5)로 확인하고, 중복 그룹을 보여줘. 삭제는 하지 마.",
        ),
        Preset::new(
            "배터리",
            "배터리 상태 확인",
            &["battery"],
            "현재 노트북 배터리 잔량(%)과 충전 상태, 가능하면 배터리 건강도를 확인해서 \
             한국어로 알려줘. macOS면 pmset -g batt 등을 활용해.",
        ),
        Preset::new(
            "압축",
            "현재 폴더를 zip으로 압축",
            &["zip"],
            "현재 작업 디렉터리를 같은 이름의 zip 파일로 압축해줘. 진행 전 만들어질 파일명을 \
             알려주고, .git 이나 node_modules, target 같은 무거운 폴더는 제외할지 물어봐.",
        ),
        Preset::new(
            "날씨",
            "원하는 지역의 실시간 날씨",
            &["weather"],
            "weather_now 도구(또는 'wonjang 날씨 [지역]' 명령)로 실시간 날씨를 가져와 \
             기온·체감·강수·최저최고를 한국어로 친근하게 알려줘. 지역이 없으면 서울.",
        ),
        Preset::new(
            "환율",
            "원/달러 등 환율 조회",
            &["fx", "exchange"],
            "현재 원/달러(USD/KRW) 환율을 web_search로 찾아 알려줘. 다른 통화가 지정되면 \
             그 통화 기준으로 조회해.",
        ),
        Preset::new(
            "일지",
            "옵시디언 오늘 일지에 기록",
            &["diary", "journal"],
            "먼저 'date +%Y-%m-%d'로 오늘 날짜를 확인하고, 옵시디언 볼트의 '일지/<오늘날짜>' \
             노트에 추가 지시로 받은 내용을 기록해줘. 'date +%H:%M'으로 시각도 앞에 붙여줘. \
             기록할 내용이 주어지지 않았으면 무엇을 적을지 사용자에게 물어봐.",
        ),
        Preset::new(
            "노트검색",
            "옵시디언 볼트에서 노트 검색",
            &["notesearch", "노트찾기"],
            "추가 지시로 받은 키워드로 옵시디언 볼트의 노트를 검색해(note_search), 관련 \
             내용을 한국어로 정리해줘. 키워드가 없으면 무엇을 찾을지 물어봐.",
        ),
        Preset::new(
            "메모",
            "옵시디언 인박스에 빠르게 메모",
            &["memo", "캡처"],
            "추가 지시로 받은 내용을 옵시디언 볼트의 '인박스' 노트에 한 줄 메모로 추가해줘\
             (note_append). 앞에 'date +%H:%M' 시각을 붙이고, 내용이 없으면 무엇을 메모할지 물어봐.",
        ),
        Preset::new(
            "번역",
            "클립보드(또는 추가 지시)의 내용을 번역",
            &["translate", "번역해줘"],
            "추가 지시로 받은 텍스트를 번역해줘. 텍스트가 없으면 클립보드 내용(read_clipboard \
             또는 pbpaste)을 가져와 번역해. 한국어면 자연스러운 영어로, 그 외 언어면 자연스러운 \
             한국어로 번역하고, 번역 결과만 깔끔히 보여줘.",
        ),
        Preset::new(
            "클립요약",
            "복사한 텍스트/링크를 요약",
            &["clipsum", "복사요약"],
            "클립보드 내용(read_clipboard 또는 pbpaste)을 가져와 한국어로 요약해줘. 내용이 \
             URL이면 web_fetch로 본문을 가져와 핵심을 불릿으로 정리하고, 일반 텍스트면 \
             3~5줄로 요약해.",
        ),
        Preset::new(
            "클립저장",
            "복사한 내용을 옵시디언 인박스에 저장",
            &["clipsave"],
            "클립보드 내용(read_clipboard 또는 pbpaste)을 가져와 옵시디언 볼트의 '인박스' \
             노트에 'date +%H:%M' 시각과 함께 저장해(note_append). 저장한 내용을 한 줄로 \
             요약해 알려줘.",
        ),
        Preset::new(
            "노션검색",
            "노션 워크스페이스에서 검색",
            &["notionsearch"],
            "추가 지시로 받은 키워드로 노션을 검색해(notion_search), 관련 페이지를 한국어로 \
             정리해줘. 키워드가 없으면 무엇을 찾을지 물어봐.",
        ),
        Preset::new(
            "노션저장",
            "노션 페이지에 메모 기록",
            &["notionsave"],
            "추가 지시로 받은 내용을 노션에 기록해줘. 먼저 적절한 대상 페이지를 notion_search로 \
             찾아 page_id를 확인하고, notion_append로 내용을 덧붙여. 대상이 모호하면 후보를 \
             보여주고 물어봐.",
        ),
        Preset::new(
            "브리핑",
            "오늘의 아침 브리핑(날짜·날씨·예정된 알림)",
            &["briefing", "아침브리핑"],
            "사용자를 위한 오늘의 아침 브리핑을 한국어로 친근하게 만들어줘. 다음을 포함해:\
             \n1) 오늘 날짜와 요일('date'로 확인)\
             \n2) 오늘 서울 날씨(weather_now 도구 또는 'wonjang 날씨'로 정확하게)\
             \n3) 예정된 약속·알림('wonjang remind' 또는 list_reminders로 확인해 가까운 순서로)\
             \n4) 오늘의 할 일('wonjang todo' 또는 list_todos로 확인)\
             \n5) 다가오는 디데이('wonjang dday' 또는 list_ddays로 확인, 가까운 것 위주)\
             \n6) 오늘 아직 안 한 습관('wonjang 습관' 또는 list_habits로 확인)\
             \n7) 짧은 응원 한마디.\
             \n간결한 불릿으로 정리해줘.",
        ),
        Preset::new(
            "회고",
            "하루를 정리하는 저녁 회고(집중·할일·지출·습관)",
            &["wrapup", "마무리", "하루정리"],
            "오늘 하루를 따뜻하게 정리하는 저녁 회고를 한국어로 만들어줘. 'wonjang 현황'으로 \
             전반을 보고, 다음을 포함해:\
             \n1) 오늘 집중 시간('wonjang 집중')\
             \n2) 완료/남은 할 일('wonjang todo')\
             \n3) 오늘 지출('wonjang 지출')\
             \n4) 체크한 습관('wonjang 습관')\
             \n5) 오늘 수고에 대한 다정한 한마디.\
             \n옵시디언 볼트가 설정돼 있으면 '일지/<오늘날짜>' 노트에 이 회고를 저장해줘.",
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

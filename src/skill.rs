//! 스킬(절차적 기억) 저장소.
//!
//! 에이전트가 어려운 작업을 해결한 뒤, 같은 일을 다시 마주쳤을 때 바로 쓸 수
//! 있도록 "방법"을 마크다운 문서로 저장한다. 메모리(memory.rs)가 *사실*을
//! 기억한다면, 스킬은 *절차*를 기억한다. 매 세션 시작 시 보유 스킬의 목록을
//! 시스템 프롬프트에 주입해, 에이전트가 필요할 때 read_skill로 펼쳐 보게 한다.
//!
//! 저장 위치: `~/.local/share/wonjang/skills/<name>.md`

use anyhow::{Context, Result};
use std::path::PathBuf;

/// 스킬 메타데이터(목록 표시용).
pub struct SkillMeta {
    pub name: String,
    pub description: String,
}

pub struct SkillStore {
    dir: PathBuf,
}

impl SkillStore {
    /// 스킬 저장소를 연다(디렉터리가 없으면 생성).
    pub fn load() -> Result<Self> {
        let dir = dirs::data_dir()
            .context("데이터 디렉터리를 찾을 수 없습니다")?
            .join("wonjang")
            .join("skills");
        std::fs::create_dir_all(&dir).ok();
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    fn path_for(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}.md", sanitize(name)))
    }

    /// 스킬을 저장한다(같은 이름은 덮어씀).
    pub fn save(&self, name: &str, description: &str, content: &str) -> Result<PathBuf> {
        let name = name.trim();
        if name.is_empty() {
            anyhow::bail!("스킬 이름이 비어 있습니다");
        }
        let doc = format!(
            "---\nname: {}\ndescription: {}\n---\n\n{}\n",
            name,
            description.trim(),
            content.trim()
        );
        let path = self.path_for(name);
        std::fs::write(&path, doc)
            .with_context(|| format!("스킬을 저장할 수 없습니다: {}", path.display()))?;
        Ok(path)
    }

    /// 이름으로 스킬 본문 전체를 읽는다.
    pub fn read(&self, name: &str) -> Result<String> {
        let path = self.path_for(name);
        std::fs::read_to_string(&path).with_context(|| format!("'{name}' 스킬을 찾을 수 없습니다"))
    }

    /// 저장된 스킬 메타데이터 목록(이름순).
    pub fn list(&self) -> Result<Vec<SkillMeta>> {
        let mut skills = Vec::new();
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(_) => return Ok(skills),
        };
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                skills.push(parse_meta(&text, &path));
            }
        }
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skills)
    }

    /// 시스템 프롬프트에 주입할 스킬 목록 블록(없으면 None).
    pub fn prompt_block(&self) -> Option<String> {
        let skills = self.list().ok()?;
        if skills.is_empty() {
            return None;
        }
        let lines: Vec<String> = skills
            .iter()
            .map(|s| format!("- {} — {}", s.name, s.description))
            .collect();
        Some(format!(
            "보유한 스킬(절차 지식). 관련 작업 전에 read_skill로 펼쳐 참고하세요:\n{}",
            lines.join("\n")
        ))
    }
}

/// 파일명으로 안전한 슬러그(한글/영숫자/-_ 유지, 나머지는 -).
fn sanitize(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = s.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "skill".to_string()
    } else {
        trimmed
    }
}

/// 마크다운 프론트매터에서 name/description을 추출(없으면 파일명/빈값).
fn parse_meta(text: &str, path: &std::path::Path) -> SkillMeta {
    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("skill")
        .to_string();
    let mut name = fallback_name.clone();
    let mut description = String::new();

    if let Some(rest) = text.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            let front = &rest[..end];
            for line in front.lines() {
                if let Some(v) = line.strip_prefix("name:") {
                    let v = v.trim();
                    if !v.is_empty() {
                        name = v.to_string();
                    }
                } else if let Some(v) = line.strip_prefix("description:") {
                    description = v.trim().to_string();
                }
            }
        }
    }
    SkillMeta { name, description }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_korean_and_alnum() {
        assert_eq!(sanitize("git 푸시 자동화!"), "git-푸시-자동화");
        assert_eq!(sanitize("  ///  "), "skill");
    }

    #[test]
    fn parse_meta_reads_frontmatter() {
        let text = "---\nname: 테스트 스킬\ndescription: 한 줄 설명\n---\n\n본문";
        let meta = parse_meta(text, std::path::Path::new("/x/foo.md"));
        assert_eq!(meta.name, "테스트 스킬");
        assert_eq!(meta.description, "한 줄 설명");
    }

    #[test]
    fn parse_meta_falls_back_to_filename() {
        let meta = parse_meta("프론트매터 없음", std::path::Path::new("/x/my-skill.md"));
        assert_eq!(meta.name, "my-skill");
        assert!(meta.description.is_empty());
    }
}

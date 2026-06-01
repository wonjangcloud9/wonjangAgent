//! GitHub 저장소 정보 — 별·이슈·최신 릴리스를 실시간으로(무료, 키 불필요).
//!
//! 개발자가 관심 저장소(또는 자기 프로젝트)의 상태를 빠르게 볼 때 쓴다.
//! GitHub 공개 REST API(인증 없이 시간당 60회)를 사용한다.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Repo {
    pub full_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub stargazers_count: u64,
    #[serde(default)]
    pub forks_count: u64,
    #[serde(default)]
    pub open_issues_count: u64,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub pushed_at: Option<String>,
}

#[derive(Deserialize)]
pub struct Release {
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("wonjang-agent")
        .build()
        .context("HTTP 클라이언트 생성 실패")
}

/// "owner/repo" 형식을 검증해 (owner, repo)로 나눈다.
pub fn split_slug(slug: &str) -> Result<(&str, &str)> {
    let s = slug.trim().trim_start_matches("https://github.com/");
    let s = s.trim_end_matches('/');
    match s.split_once('/') {
        Some((o, r)) if !o.is_empty() && !r.is_empty() && !r.contains('/') => Ok((o, r)),
        _ => Err(anyhow!(
            "owner/repo 형식으로 입력하세요 (예: rust-lang/rust)"
        )),
    }
}

/// 저장소 정보를 가져온다.
pub async fn fetch_repo(owner: &str, repo: &str) -> Result<Repo> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let resp = client()?
        .get(&url)
        .send()
        .await
        .context("GitHub 요청 실패")?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("저장소를 찾을 수 없어요: {owner}/{repo}"));
    }
    if !resp.status().is_success() {
        return Err(anyhow!("GitHub 응답 오류: {}", resp.status()));
    }
    resp.json().await.context("GitHub 응답 파싱 실패")
}

/// 최신 릴리스를 가져온다(없으면 None).
pub async fn fetch_latest_release(owner: &str, repo: &str) -> Option<Release> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let resp = client().ok()?.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json().await.ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_owner_repo() {
        assert_eq!(split_slug("rust-lang/rust").unwrap(), ("rust-lang", "rust"));
        assert_eq!(split_slug("https://github.com/a/b/").unwrap(), ("a", "b"));
        assert!(split_slug("invalid").is_err());
        assert!(split_slug("a/b/c").is_err());
    }
}

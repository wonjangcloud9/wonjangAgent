//! 메시징 게이트웨이 — 텔레그램 봇.
//!
//! 텔레그램 롱폴링(getUpdates)으로 메시지를 받아 에이전트에 전달하고, 결과를
//! 다시 메시지로 돌려준다. 어디서든(휴대폰 등) 원장에게 작업을 시킬 수 있다.
//!
//! ⚠️ 보안: 원격에서 셸 실행까지 가능하므로, `telegram_allowed_ids`에 등록된
//! chat_id만 실제 작업을 수행한다. 목록이 비어 있으면 누구의 요청도 실행하지
//! 않고, 본인 chat_id를 알려줘 등록을 돕는다.

use crate::agent;
use crate::config::Config;
use crate::engine::Engine;
use crate::llm::Message;
use crate::memory::Memory;
use crate::skill::SkillStore;
use crate::tools::ToolContext;
use crate::ui;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Deserialize)]
struct UpdatesResp {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    result: Vec<Update>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct Update {
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Deserialize)]
struct TgMessage {
    chat: Chat,
    from: Option<TgUser>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct Chat {
    id: i64,
}

#[derive(Deserialize)]
struct TgUser {
    #[serde(default)]
    first_name: String,
}

/// 텔레그램 게이트웨이 실행(포그라운드 데몬).
pub async fn run_telegram(eng: &Engine, cfg: &Config) -> Result<()> {
    let token = cfg.telegram_token.clone();
    if token.is_empty() {
        bail!(
            "텔레그램 봇 토큰이 없습니다. 환경 변수로 설정하세요:\n  \
             export TELEGRAM_BOT_TOKEN=123456:ABC...\n\
             봇 생성은 텔레그램 @BotFather 에서 할 수 있습니다."
        );
    }

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(65))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let base = format!("https://api.telegram.org/bot{token}");

    // 봇 정보 확인(토큰 유효성 검증).
    match get_me(&http, &base).await {
        Ok(name) => ui::note(&format!(
            "텔레그램 게이트웨이 시작 — 봇 '@{name}'. 종료는 Ctrl-C."
        )),
        Err(e) => bail!("텔레그램 연결 실패(토큰 확인): {e:#}"),
    }
    if cfg.telegram_allowed_ids.is_empty() {
        ui::note(
            "⚠️ 허용된 chat_id가 없습니다. 봇에게 메시지를 보내면 본인 chat_id를 알려드립니다. \
             설정의 telegram_allowed_ids에 추가한 뒤 다시 실행하세요.",
        );
    } else {
        ui::info(&format!("허용된 chat_id: {:?}", cfg.telegram_allowed_ids));
    }

    // 원격/무인 실행 — 위험 명령은 기본 차단.
    let ctx = ToolContext {
        auto_approve: true,
        allow_dangerous: false,
    };
    let mut histories: HashMap<i64, Vec<Message>> = HashMap::new();
    let mut offset: i64 = 0;

    loop {
        let updates = match get_updates(&http, &base, offset).await {
            Ok(u) => u,
            Err(e) => {
                ui::error(&format!("getUpdates 오류: {e:#} (5초 후 재시도)"));
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        for up in updates {
            offset = up.update_id + 1;
            let Some(msg) = up.message else { continue };
            let Some(text) = msg.text else { continue };
            let chat_id = msg.chat.id;
            let who = msg.from.map(|u| u.first_name).unwrap_or_default();

            // 인증.
            if cfg.telegram_allowed_ids.is_empty() {
                let _ = send_message(
                    &http,
                    &base,
                    chat_id,
                    &format!(
                        "아직 허용된 사용자가 없습니다.\n당신의 chat_id는 `{chat_id}` 입니다.\n\
                         설정 telegram_allowed_ids 에 추가한 뒤 게이트웨이를 다시 실행해 주세요."
                    ),
                )
                .await;
                continue;
            }
            if !cfg.telegram_allowed_ids.contains(&chat_id) {
                let _ = send_message(
                    &http,
                    &base,
                    chat_id,
                    &format!("권한이 없습니다. (chat_id: {chat_id})"),
                )
                .await;
                ui::note(&format!("거부된 요청: chat_id={chat_id} ({who})"));
                continue;
            }

            ui::note(&format!("◀ [{who}/{chat_id}] {}", first_line(&text)));

            // 채팅별 대화 기록 유지(메모리/스킬은 최초에 주입).
            let history = histories.entry(chat_id).or_insert_with(|| {
                let mem = Memory::load().ok().and_then(|m| m.prompt_block());
                let skills = SkillStore::load().ok().and_then(|s| s.prompt_block());
                vec![Message::system(agent::system_prompt(mem, skills))]
            });
            history.push(Message::user(text));

            let reply = match eng.run(cfg, &ctx, history).await {
                Ok(ans) => ans.unwrap_or_else(|| "(응답을 만들지 못했습니다)".to_string()),
                Err(e) => format!("작업 중 오류가 발생했습니다: {e}"),
            };
            ui::info(&format!("▶ [{chat_id}] {}", first_line(&reply)));
            if let Err(e) = send_message(&http, &base, chat_id, &reply).await {
                ui::error(&format!("sendMessage 오류: {e:#}"));
            }
        }
    }
}

/// getMe — 봇 사용자명 반환(토큰 검증).
async fn get_me(http: &reqwest::Client, base: &str) -> Result<String> {
    let v: serde_json::Value = http
        .get(format!("{base}/getMe"))
        .send()
        .await?
        .json()
        .await?;
    if v.get("ok").and_then(|b| b.as_bool()) != Some(true) {
        bail!("응답 ok=false: {v}");
    }
    Ok(v["result"]["username"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}

/// 롱폴링으로 업데이트를 받는다.
async fn get_updates(http: &reqwest::Client, base: &str, offset: i64) -> Result<Vec<Update>> {
    let resp: UpdatesResp = http
        .get(format!("{base}/getUpdates"))
        .query(&[
            ("offset", offset.to_string()),
            ("timeout", "30".to_string()),
        ])
        .send()
        .await?
        .json()
        .await
        .context("getUpdates 응답 파싱 실패")?;
    // API 오류(예: 409 Conflict — 같은 토큰으로 다른 폴러)는 ok=false로 즉시 반환된다.
    // 이를 Ok(빈 결과)로 흘리면 메인 루프가 sleep 없이 무한 핫루프에 빠지므로 Err로.
    if !resp.ok {
        bail!(
            "텔레그램 API 오류: {}",
            resp.description.as_deref().unwrap_or("ok=false")
        );
    }
    Ok(resp.result)
}

/// 메시지를 보낸다(텔레그램 4096자 제한 고려해 분할).
async fn send_message(http: &reqwest::Client, base: &str, chat_id: i64, text: &str) -> Result<()> {
    for chunk in split_chunks(text, 4000) {
        http.post(format!("{base}/sendMessage"))
            .json(&serde_json::json!({ "chat_id": chat_id, "text": chunk }))
            .send()
            .await?
            .error_for_status()?;
    }
    Ok(())
}

/// 긴 텍스트를 최대 길이 단위로 분할(문자 경계 안전).
fn split_chunks(text: &str, max: usize) -> Vec<String> {
    if text.is_empty() {
        return vec!["(빈 응답)".to_string()];
    }
    let mut chunks = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if cur.chars().count() >= max {
            chunks.push(std::mem::take(&mut cur));
        }
        cur.push(ch);
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_short_text_single_chunk() {
        assert_eq!(split_chunks("안녕하세요", 100), vec!["안녕하세요"]);
    }

    #[test]
    fn split_long_text_multiple_chunks() {
        let text = "가".repeat(250);
        let chunks = split_chunks(&text, 100);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chars().count(), 100);
        assert_eq!(chunks[2].chars().count(), 50);
    }

    #[test]
    fn empty_text_yields_placeholder() {
        assert_eq!(split_chunks("", 100), vec!["(빈 응답)"]);
    }

    #[test]
    fn updates_resp_captures_api_error() {
        // 409 Conflict 등 API 오류 바디는 ok=false로 잡혀야(get_updates가 Err로 → 핫루프 방지).
        let err: UpdatesResp =
            serde_json::from_str(r#"{"ok":false,"error_code":409,"description":"Conflict"}"#)
                .unwrap();
        assert!(!err.ok);
        assert_eq!(err.description.as_deref(), Some("Conflict"));
        assert!(err.result.is_empty());
        // 정상 응답은 ok=true.
        let okr: UpdatesResp = serde_json::from_str(r#"{"ok":true,"result":[]}"#).unwrap();
        assert!(okr.ok);
    }
}

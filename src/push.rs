//! 푸시 알림 — 외출 중에도 휴대폰으로 알림을 받는다.
//!
//! 설정된 채널(디스코드 웹훅 / 텔레그램)로 메시지를 보낸다. 24시간 비서가
//! 약속·알림이 울릴 때 데스크탑 알림과 함께 이 채널로도 푸시해, 자리를 비워도
//! 놓치지 않게 한다.

use crate::config::Config;
use anyhow::Result;
use std::time::Duration;

/// 설정된 푸시 채널 이름 목록(상태 표시·테스트용).
pub fn configured_channels(cfg: &Config) -> Vec<&'static str> {
    let mut v = Vec::new();
    if !cfg.discord_webhook.trim().is_empty() {
        v.push("discord");
    }
    if !cfg.slack_webhook.trim().is_empty() {
        v.push("slack");
    }
    if !cfg.telegram_token.trim().is_empty() && !cfg.telegram_allowed_ids.is_empty() {
        v.push("telegram");
    }
    if !cfg.kakao_access_token.trim().is_empty() {
        v.push("kakao");
    }
    v
}

/// 설정된 모든 채널로 메시지를 보낸다(보낸 채널 수 반환).
pub async fn push(cfg: &Config, message: &str) -> usize {
    let mut sent = 0;
    let http = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return 0,
    };

    if !cfg.discord_webhook.trim().is_empty()
        && discord_send(&http, &cfg.discord_webhook, message)
            .await
            .is_ok()
    {
        sent += 1;
    }

    if !cfg.slack_webhook.trim().is_empty()
        && slack_send(&http, &cfg.slack_webhook, message).await.is_ok()
    {
        sent += 1;
    }

    if !cfg.telegram_token.trim().is_empty() {
        if let Some(chat) = cfg.telegram_allowed_ids.first() {
            if telegram_send(&http, &cfg.telegram_token, *chat, message)
                .await
                .is_ok()
            {
                sent += 1;
            }
        }
    }

    if !cfg.kakao_access_token.trim().is_empty()
        && kakao_send(&http, &cfg.kakao_access_token, message)
            .await
            .is_ok()
    {
        sent += 1;
    }
    sent
}

/// 동기 컨텍스트(서브커맨드·데몬)에서 푸시한다.
pub fn push_blocking(cfg: &Config, message: &str) -> usize {
    let cfg = cfg.clone();
    let msg = message.to_string();
    crate::util::run_async(async move { Ok(push(&cfg, &msg).await) }).unwrap_or(0)
}

async fn discord_send(http: &reqwest::Client, webhook: &str, message: &str) -> Result<()> {
    http.post(webhook)
        .json(&serde_json::json!({ "content": message }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// 슬랙 Incoming Webhook으로 보낸다(POST {"text": ...}). 한국 회사에서 흔한 채널.
async fn slack_send(http: &reqwest::Client, webhook: &str, message: &str) -> Result<()> {
    http.post(webhook)
        .json(&serde_json::json!({ "text": message }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn telegram_send(
    http: &reqwest::Client,
    token: &str,
    chat_id: i64,
    message: &str,
) -> Result<()> {
    http.post(format!("https://api.telegram.org/bot{token}/sendMessage"))
        .json(&serde_json::json!({ "chat_id": chat_id, "text": message }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// 카카오톡 '나에게 보내기'(메모 API)로 푸시한다.
async fn kakao_send(http: &reqwest::Client, token: &str, message: &str) -> Result<()> {
    let template = serde_json::json!({
        "object_type": "text",
        "text": message,
        "link": { "web_url": "https://github.com/wonjangcloud9/wonjangAgent" }
    })
    .to_string();
    http.post("https://kapi.kakao.com/v2/api/talk/memo/default/send")
        .bearer_auth(token)
        .form(&[("template_object", template.as_str())])
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_detected() {
        let mut cfg = Config::default();
        assert!(configured_channels(&cfg).is_empty());
        cfg.discord_webhook = "https://discord.com/api/webhooks/x".into();
        assert_eq!(configured_channels(&cfg), vec!["discord"]);
        cfg.slack_webhook = "https://hooks.slack.com/services/x".into();
        assert_eq!(configured_channels(&cfg), vec!["discord", "slack"]);
        cfg.telegram_token = "t".into();
        cfg.telegram_allowed_ids = vec![1];
        assert_eq!(
            configured_channels(&cfg),
            vec!["discord", "slack", "telegram"]
        );
        cfg.kakao_access_token = "k".into();
        assert_eq!(
            configured_channels(&cfg),
            vec!["discord", "slack", "telegram", "kakao"]
        );
    }
}

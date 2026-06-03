//! 이메일 읽기 — IMAP over rustls.
//!
//! 자격증명은 환경변수로만 받는다(설정 파일에 비밀번호를 저장하지 않음):
//! - `WONJANG_EMAIL`          : 이메일 주소(아이디)
//! - `WONJANG_EMAIL_PASSWORD` : 앱 비밀번호(2단계 인증 계정의 앱 비밀번호 권장)
//! - `WONJANG_EMAIL_HOST`     : (선택) IMAP 호스트. 생략 시 도메인으로 추정
//! - `WONJANG_EMAIL_PORT`     : (선택) IMAP 포트(기본 993)
//!
//! GPT가 못 하는 일: 사용자의 실제 메일함에 접속해 받은편지함을 읽는다.

use anyhow::{Context, Result};
use base64::Engine;
use std::net::TcpStream;
use std::sync::Arc;

/// 환경변수에서 읽은 이메일 접속 설정.
pub struct EmailConfig {
    pub host: String,
    pub port: u16,
    pub smtp_host: String,
    pub user: String,
    pub password: String,
}

impl EmailConfig {
    /// 환경변수에서 설정을 읽는다(없으면 None).
    pub fn from_env() -> Option<Self> {
        let user = std::env::var("WONJANG_EMAIL").ok()?;
        let password = std::env::var("WONJANG_EMAIL_PASSWORD").ok()?;
        if user.trim().is_empty() || password.is_empty() {
            return None;
        }
        let host = std::env::var("WONJANG_EMAIL_HOST")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| guess_imap_host(&user));
        let port = std::env::var("WONJANG_EMAIL_PORT")
            .ok()
            .and_then(|p| p.trim().parse().ok())
            .unwrap_or(993);
        let smtp_host = std::env::var("WONJANG_SMTP_HOST")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| guess_smtp_host(&user));
        Some(Self {
            host,
            port,
            smtp_host,
            user,
            password,
        })
    }
}

/// 이메일 주소의 도메인으로 IMAP 호스트를 추정한다(흔한 한국·글로벌 제공자). 순수.
pub fn guess_imap_host(email: &str) -> String {
    let domain = email.split('@').nth(1).unwrap_or("").to_lowercase();
    match domain.as_str() {
        "gmail.com" | "googlemail.com" => "imap.gmail.com".to_string(),
        "naver.com" => "imap.naver.com".to_string(),
        "daum.net" | "hanmail.net" => "imap.daum.net".to_string(),
        "kakao.com" => "imap.kakao.com".to_string(),
        "nate.com" => "imap.mail.nate.com".to_string(),
        "outlook.com" | "hotmail.com" | "live.com" | "msn.com" => {
            "outlook.office365.com".to_string()
        }
        "icloud.com" | "me.com" => "imap.mail.me.com".to_string(),
        "yahoo.com" => "imap.mail.yahoo.com".to_string(),
        "" => "imap.localhost".to_string(),
        other => format!("imap.{other}"),
    }
}

/// 이메일 도메인으로 SMTP 호스트를 추정한다(보내기용). 순수.
pub fn guess_smtp_host(email: &str) -> String {
    let domain = email.split('@').nth(1).unwrap_or("").to_lowercase();
    match domain.as_str() {
        "gmail.com" | "googlemail.com" => "smtp.gmail.com".to_string(),
        "naver.com" => "smtp.naver.com".to_string(),
        "daum.net" | "hanmail.net" => "smtp.daum.net".to_string(),
        "kakao.com" => "smtp.kakao.com".to_string(),
        "nate.com" => "smtp.mail.nate.com".to_string(),
        "outlook.com" | "hotmail.com" | "live.com" | "msn.com" => "smtp.office365.com".to_string(),
        "icloud.com" | "me.com" => "smtp.mail.me.com".to_string(),
        "yahoo.com" => "smtp.mail.yahoo.com".to_string(),
        "" => "smtp.localhost".to_string(),
        other => format!("smtp.{other}"),
    }
}

/// 메일을 보낸다(SMTP over rustls, 암묵 TLS 465). 외부 전송 — 호출자가 사용자 의도를 확인한 뒤 부른다.
pub fn send_mail(cfg: &EmailConfig, to: &str, subject: &str, body: &str) -> Result<()> {
    use lettre::message::header::ContentType;
    use lettre::message::Mailbox;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    let from: Mailbox = cfg
        .user
        .parse()
        .map_err(|_| anyhow::anyhow!("보내는 주소 형식이 이상해요: {}", cfg.user))?;
    let to_mb: Mailbox = to
        .parse()
        .map_err(|_| anyhow::anyhow!("받는 사람 주소 형식이 올바르지 않아요: {to}"))?;
    let email = Message::builder()
        .from(from)
        .to(to_mb)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| anyhow::anyhow!("메일 작성 실패: {e}"))?;
    let creds = Credentials::new(cfg.user.clone(), cfg.password.clone());
    let mailer = SmtpTransport::relay(&cfg.smtp_host)
        .map_err(|e| anyhow::anyhow!("SMTP 설정 실패({}): {e}", cfg.smtp_host))?
        .credentials(creds)
        .build();
    mailer
        .send(&email)
        .map_err(|e| anyhow::anyhow!("전송 실패: {e}. 앱 비밀번호/SMTP 설정을 확인하세요."))?;
    Ok(())
}

/// 받은편지함 헤더 한 줄.
pub struct MailHeader {
    pub seq: u32,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub unseen: bool,
}

/// 조회 결과.
pub struct InboxView {
    pub total: u32,
    pub unseen: usize,
    pub headers: Vec<MailHeader>,
}

/// rustls TLS 스트림을 만든다(ring provider 명시 — provider 충돌 방지).
fn tls_stream(
    host: &str,
    port: u16,
) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>> {
    let root_store = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .context("TLS 설정 실패")?
    .with_root_certificates(root_store)
    .with_no_client_auth();
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|_| anyhow::anyhow!("호스트 이름이 올바르지 않아요: {host}"))?;
    let conn = rustls::ClientConnection::new(Arc::new(config), server_name)?;
    let sock = TcpStream::connect((host, port))
        .with_context(|| format!("{host}:{port} 연결 실패(네트워크/방화벽 확인)"))?;
    Ok(rustls::StreamOwned::new(conn, sock))
}

/// 받은편지함의 최근 메일(또는 안 읽은 메일)을 가져온다.
pub fn fetch_inbox(cfg: &EmailConfig, count: usize, unseen_only: bool) -> Result<InboxView> {
    use std::collections::HashSet;

    let tls = tls_stream(&cfg.host, cfg.port)?;
    let client = imap::Client::new(tls);
    let mut session = client.login(&cfg.user, &cfg.password).map_err(|(e, _)| {
        anyhow::anyhow!("로그인 실패: {e}. 아이디/앱 비밀번호를 확인하세요(보통 일반 비밀번호가 아니라 '앱 비밀번호'가 필요해요).")
    })?;

    let mailbox = session.select("INBOX")?;
    let total = mailbox.exists;
    let unseen_set: HashSet<u32> = session.search("UNSEEN").unwrap_or_default();

    let ids: Vec<u32> = if unseen_only {
        let mut v: Vec<u32> = unseen_set.iter().copied().collect();
        v.sort_unstable();
        v
    } else if total == 0 {
        vec![]
    } else {
        let start = total.saturating_sub(count as u32).saturating_add(1).max(1);
        (start..=total).collect()
    };

    let mut headers = Vec::new();
    if !ids.is_empty() {
        let set = ids
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let fetches = session.fetch(set, "(ENVELOPE FLAGS)")?;
        for f in fetches.iter() {
            let seq = f.message;
            let env = f.envelope();
            let from = env
                .and_then(|e| e.from.as_ref())
                .and_then(|v| v.first())
                .map(format_address)
                .unwrap_or_else(|| "(보낸이 없음)".to_string());
            let subject = env
                .and_then(|e| e.subject)
                .map(|b| decode_mime_words(&String::from_utf8_lossy(b)))
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "(제목 없음)".to_string());
            let date = env
                .and_then(|e| e.date)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            headers.push(MailHeader {
                seq,
                from,
                subject,
                date,
                unseen: unseen_set.contains(&seq),
            });
        }
        headers.sort_by_key(|b| std::cmp::Reverse(b.seq)); // 최신 먼저
    }

    let _ = session.logout();
    Ok(InboxView {
        total,
        unseen: unseen_set.len(),
        headers,
    })
}

/// 본문까지 포함한 한 통.
pub struct MailFull {
    pub from: String,
    pub subject: String,
    pub date: String,
    pub body: String,
}

/// 받은편지함에서 num번째(1=최신) 메일의 본문까지 읽는다. unseen_only면 안 읽은 것 중에서.
pub fn fetch_message(cfg: &EmailConfig, num: usize, unseen_only: bool) -> Result<Option<MailFull>> {
    use mailparse::MailHeaderMap;
    use std::collections::HashSet;

    let tls = tls_stream(&cfg.host, cfg.port)?;
    let client = imap::Client::new(tls);
    let mut session = client
        .login(&cfg.user, &cfg.password)
        .map_err(|(e, _)| anyhow::anyhow!("로그인 실패: {e}. 아이디/앱 비밀번호를 확인하세요."))?;
    let mailbox = session.select("INBOX")?;
    let total = mailbox.exists;
    let unseen_set: HashSet<u32> = session.search("UNSEEN").unwrap_or_default();

    // 최신(큰 seq) 우선 후보 목록.
    let ordered: Vec<u32> = if unseen_only {
        let mut v: Vec<u32> = unseen_set.iter().copied().collect();
        v.sort_unstable_by(|a, b| b.cmp(a));
        v
    } else {
        (1..=total).rev().collect()
    };
    let idx = num.max(1) - 1;
    let seq = match ordered.get(idx) {
        Some(s) => *s,
        None => {
            let _ = session.logout();
            return Ok(None);
        }
    };

    let fetches = session.fetch(seq.to_string(), "RFC822")?;
    let raw = fetches
        .iter()
        .next()
        .and_then(|f| f.body())
        .map(|b| b.to_vec());
    let _ = session.logout();
    let raw = match raw {
        Some(r) => r,
        None => return Ok(None),
    };

    let parsed = mailparse::parse_mail(&raw).context("메일을 해석하지 못했어요")?;
    let subject = parsed
        .headers
        .get_first_value("Subject")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "(제목 없음)".to_string());
    let from = parsed.headers.get_first_value("From").unwrap_or_default();
    let date = parsed.headers.get_first_value("Date").unwrap_or_default();
    let body = extract_readable_body(&parsed);
    Ok(Some(MailFull {
        from,
        subject,
        date,
        body,
    }))
}

/// 멀티파트 메일에서 사람이 읽을 본문을 고른다(text/plain 우선 → text/html 태그제거 → 첫 본문).
fn extract_readable_body(parsed: &mailparse::ParsedMail) -> String {
    fn walk(p: &mailparse::ParsedMail, want_plain: bool) -> Option<String> {
        if p.subparts.is_empty() {
            let mt = p.ctype.mimetype.to_lowercase();
            if want_plain && mt == "text/plain" {
                return p.get_body().ok();
            }
            if !want_plain && mt == "text/html" {
                return p.get_body().ok().map(|h| strip_html(&h));
            }
            return None;
        }
        for sp in &p.subparts {
            if let Some(b) = walk(sp, want_plain) {
                return Some(b);
            }
        }
        None
    }
    let body = walk(parsed, true)
        .or_else(|| walk(parsed, false))
        .or_else(|| parsed.get_body().ok())
        .unwrap_or_default();
    // 과한 빈 줄 정리.
    let mut out = String::new();
    let mut blanks = 0;
    for line in body.lines() {
        let t = line.trim_end();
        if t.is_empty() {
            blanks += 1;
            if blanks > 1 {
                continue;
            }
        } else {
            blanks = 0;
        }
        out.push_str(t);
        out.push('\n');
    }
    out.trim().to_string()
}

/// 아주 단순한 HTML 태그 제거 + 기본 엔티티 복원(본문 미리보기용). 순수.
fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// IMAP Address를 "이름 <메일주소>" 문자열로(이름은 MIME 디코드).
fn format_address(a: &imap_proto::Address) -> String {
    let name = a
        .name
        .map(|b| decode_mime_words(&String::from_utf8_lossy(b)))
        .filter(|s| !s.trim().is_empty());
    let mailbox = a.mailbox.map(|b| String::from_utf8_lossy(b).into_owned());
    let host = a.host.map(|b| String::from_utf8_lossy(b).into_owned());
    let addr = match (mailbox, host) {
        (Some(m), Some(h)) => format!("{m}@{h}"),
        (Some(m), None) => m,
        _ => String::new(),
    };
    match name {
        Some(n) if !addr.is_empty() => format!("{n} <{addr}>"),
        Some(n) => n,
        None => addr,
    }
}

/// RFC2047 인코딩된 단어(`=?charset?B/Q?text?=`)를 사람이 읽는 문자열로 디코드한다. 순수.
/// 인코딩된 단어 사이의 공백은 무시한다(RFC2047 규칙). 한글은 UTF-8·EUC-KR 모두 지원.
pub fn decode_mime_words(input: &str) -> String {
    let mut result = String::new();
    let mut remainder = input;
    let mut prev_encoded = false;
    loop {
        match remainder.find("=?") {
            None => {
                result.push_str(remainder);
                break;
            }
            Some(start) => {
                let plain = &remainder[..start];
                // 인코딩된 단어 사이에 공백만 있으면 버린다.
                if !(prev_encoded && plain.trim().is_empty()) {
                    result.push_str(plain);
                }
                let after = &remainder[start..];
                if let Some((decoded, consumed)) = decode_one_word(after) {
                    result.push_str(&decoded);
                    remainder = &after[consumed..];
                    prev_encoded = true;
                } else {
                    result.push_str("=?");
                    remainder = &after[2..];
                    prev_encoded = false;
                }
            }
        }
    }
    result
}

/// `=?`로 시작하는 한 개의 인코딩 단어를 디코드한다. (디코드 결과, 소비한 바이트 수).
fn decode_one_word(s: &str) -> Option<(String, usize)> {
    let body = s.strip_prefix("=?")?;
    let q1 = body.find('?')?;
    let charset = &body[..q1];
    let rest = &body[q1 + 1..];
    let q2 = rest.find('?')?;
    let enc = &rest[..q2];
    let rest2 = &rest[q2 + 1..];
    let end = rest2.find("?=")?;
    let text = &rest2[..end];
    let consumed = 2 + q1 + 1 + q2 + 1 + end + 2;
    let raw = match enc.to_ascii_uppercase().as_str() {
        "B" => base64::engine::general_purpose::STANDARD
            .decode(text)
            .ok()?,
        "Q" => q_decode(text),
        _ => return None,
    };
    Some((decode_charset(charset, &raw), consumed))
}

/// Q-인코딩(quoted-printable 변형): `_`는 공백, `=XX`는 16진 바이트.
fn q_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'_' => {
                out.push(b' ');
                i += 1;
            }
            b'=' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    out.push(v);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    out
}

/// charset 라벨에 따라 바이트를 문자열로(UTF-8·EUC-KR/CP949 등).
fn decode_charset(charset: &str, bytes: &[u8]) -> String {
    let label = charset.to_ascii_lowercase();
    match label.as_str() {
        "utf-8" | "utf8" | "us-ascii" | "ascii" => String::from_utf8_lossy(bytes).into_owned(),
        "euc-kr" | "ks_c_5601-1987" | "ksc5601" | "cp949" | "uhc" => {
            encoding_rs::EUC_KR.decode(bytes).0.into_owned()
        }
        _ => match encoding_rs::Encoding::for_label(label.as_bytes()) {
            Some(enc) => enc.decode(bytes).0.into_owned(),
            None => String::from_utf8_lossy(bytes).into_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guesses_common_hosts() {
        assert_eq!(guess_imap_host("me@gmail.com"), "imap.gmail.com");
        assert_eq!(guess_imap_host("me@naver.com"), "imap.naver.com");
        assert_eq!(guess_imap_host("me@daum.net"), "imap.daum.net");
        assert_eq!(guess_imap_host("me@company.co.kr"), "imap.company.co.kr");
    }

    #[test]
    fn guesses_smtp_hosts() {
        assert_eq!(guess_smtp_host("me@gmail.com"), "smtp.gmail.com");
        assert_eq!(guess_smtp_host("me@naver.com"), "smtp.naver.com");
        assert_eq!(guess_smtp_host("me@outlook.com"), "smtp.office365.com");
        assert_eq!(guess_smtp_host("me@company.co.kr"), "smtp.company.co.kr");
    }

    #[test]
    fn decodes_utf8_base64_word() {
        // "=?UTF-8?B?7JWI64WV?=" == "안녕"
        let enc = "=?UTF-8?B?7JWI64WV?=";
        assert_eq!(decode_mime_words(enc), "안녕");
    }

    #[test]
    fn decodes_euckr_and_q_encoding() {
        // Q-인코딩: "Hi_World" → "Hi World"
        assert_eq!(decode_mime_words("=?UTF-8?Q?Hi_World?="), "Hi World");
        // EUC-KR base64 "한글" = b0a1... 실제로 인코딩해서 검증.
        let euckr = encoding_rs::EUC_KR.encode("한글").0;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&euckr);
        let word = format!("=?EUC-KR?B?{b64}?=");
        assert_eq!(decode_mime_words(&word), "한글");
    }

    #[test]
    fn strips_html_and_entities() {
        assert_eq!(strip_html("<p>안녕&nbsp;<b>세상</b></p>"), "안녕 세상");
        assert_eq!(strip_html("a &lt;b&gt; &amp; c"), "a <b> & c");
    }

    #[test]
    fn extracts_plain_part_preferred() {
        // multipart/alternative: text/plain(한글) 우선 선택.
        let eml = b"From: a@b.com\r\nSubject: t\r\nContent-Type: multipart/alternative; boundary=\"bb\"\r\n\r\n--bb\r\nContent-Type: text/plain; charset=\"UTF-8\"\r\n\r\n\xec\x95\x88\xeb\x85\x95 \xeb\xb3\xb8\xeb\xac\xb8\r\n--bb\r\nContent-Type: text/html; charset=\"UTF-8\"\r\n\r\n<p>html</p>\r\n--bb--\r\n";
        let parsed = mailparse::parse_mail(eml).unwrap();
        assert_eq!(super::extract_readable_body(&parsed), "안녕 본문");
    }

    #[test]
    fn falls_back_to_html_when_no_plain() {
        let eml = b"Subject: t\r\nContent-Type: text/html; charset=\"UTF-8\"\r\n\r\n<div>\xed\x95\x9c\xea\xb8\x80 <b>html</b></div>\r\n";
        let parsed = mailparse::parse_mail(eml).unwrap();
        assert_eq!(super::extract_readable_body(&parsed), "한글 html");
    }

    #[test]
    fn passes_through_plain_text_and_joins_words() {
        assert_eq!(decode_mime_words("그냥 제목"), "그냥 제목");
        // 인코딩된 단어 사이 공백은 무시.
        let a = "=?UTF-8?B?7JWI?="; // "안"
        let b = "=?UTF-8?B?64WV?="; // "녕"
        assert_eq!(decode_mime_words(&format!("{a} {b}")), "안녕");
        // 일반 텍스트 + 인코딩 단어 혼합.
        assert_eq!(decode_mime_words(&format!("[알림] {a}")), "[알림] 안");
    }
}

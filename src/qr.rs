//! QR 코드 생성 — 와이파이·링크·송금 정보를 터미널에 바로 스캔 가능한 QR로.
//!
//! 유니코드 반블록(▀▄█)으로 그려, 별도 이미지 없이 터미널에서 휴대폰으로 스캔할
//! 수 있다. 외부 의존성은 qrcode 크레이트뿐이고 네트워크·키가 필요 없다.

use anyhow::{anyhow, Result};
use qrcode::render::unicode;
use qrcode::QrCode;

/// 문자열을 터미널용 QR(유니코드 반블록 문자열)로 만든다.
pub fn render_terminal(data: &str) -> Result<String> {
    if data.trim().is_empty() {
        return Err(anyhow!("QR로 만들 내용을 입력하세요"));
    }
    let code = QrCode::new(data.as_bytes()).map_err(|e| anyhow!("QR 생성 실패: {e}"))?;
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .quiet_zone(true)
        .build();
    Ok(image)
}

/// 와이파이 접속용 QR 문자열 규격(WIFI:T:WPA;S:<ssid>;P:<pw>;;).
pub fn wifi_payload(ssid: &str, password: &str) -> String {
    let esc = |s: &str| {
        s.replace('\\', "\\\\")
            .replace(';', "\\;")
            .replace(':', "\\:")
    };
    if password.is_empty() {
        format!("WIFI:T:nopass;S:{};;", esc(ssid))
    } else {
        format!("WIFI:T:WPA;S:{};P:{};;", esc(ssid), esc(password))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_non_empty() {
        let s = render_terminal("https://example.com").unwrap();
        assert!(s.lines().count() > 5);
        assert!(s
            .chars()
            .any(|c| c == '█' || c == '▀' || c == '▄' || c == ' '));
    }

    #[test]
    fn empty_errors() {
        assert!(render_terminal("   ").is_err());
    }

    #[test]
    fn wifi_payload_format() {
        assert_eq!(
            wifi_payload("MyWiFi", "pass123"),
            "WIFI:T:WPA;S:MyWiFi;P:pass123;;"
        );
        assert_eq!(wifi_payload("Open", ""), "WIFI:T:nopass;S:Open;;");
        // 특수문자 이스케이프.
        assert_eq!(wifi_payload("a;b", "c:d"), "WIFI:T:WPA;S:a\\;b;P:c\\:d;;");
    }
}

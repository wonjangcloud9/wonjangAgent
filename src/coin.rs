//! 코인 시세 — 한국 1위 거래소 업비트의 실시간 시세(무료, 키 불필요).

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug)]
pub struct Coin {
    pub symbol: String,
    pub price: f64,
    /// 전일 대비 변동률(%, 부호 포함).
    pub change_pct: f64,
}

#[derive(Deserialize)]
struct Ticker {
    market: String,
    trade_price: f64,
    signed_change_rate: f64,
}

/// 마켓 코드 목록(예: ["KRW-BTC"])의 시세를 가져온다.
pub async fn fetch(markets: &[String]) -> Result<Vec<Coin>> {
    let q = markets.join(",");
    let url = format!("https://api.upbit.com/v1/ticker?markets={q}");
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;
    let text = http
        .get(&url)
        .send()
        .await
        .context("코인 시세 요청 실패")?
        .text()
        .await
        .context("코인 시세 응답 읽기 실패")?;
    parse_tickers(&text, markets)
}

/// 업비트 ticker 응답(JSON 배열)을 파싱한다.
///
/// 잘못된 심볼이면 업비트가 배열 대신 에러 객체(`{"error":{...}}`)를 돌려주는데,
/// 그대로 `Vec<Ticker>`로 역직렬화하면 serde 내부 오류(영문 "invalid type: map,
/// expected a sequence")가 사용자에게 새어 나간다. 배열로 안 풀리면 내부 오류를
/// 감추고 깔끔한 한국어 안내로 바꾼다.
fn parse_tickers(text: &str, markets: &[String]) -> Result<Vec<Coin>> {
    if let Ok(tickers) = serde_json::from_str::<Vec<Ticker>>(text) {
        if !tickers.is_empty() {
            return Ok(tickers
                .into_iter()
                .map(|t| Coin {
                    symbol: t
                        .market
                        .strip_prefix("KRW-")
                        .unwrap_or(&t.market)
                        .to_string(),
                    price: t.trade_price,
                    change_pct: t.signed_change_rate * 100.0,
                })
                .collect());
        }
    }
    let symbols: Vec<String> = markets
        .iter()
        .map(|m| m.strip_prefix("KRW-").unwrap_or(m).to_string())
        .collect();
    bail!(
        "'{}' 코인을 찾을 수 없어요. 심볼을 확인해 주세요 (예: BTC, ETH, XRP)",
        symbols.join(", ")
    );
}

/// 기본으로 보여줄 인기 코인 마켓.
pub fn default_markets() -> Vec<String> {
    ["KRW-BTC", "KRW-ETH", "KRW-XRP", "KRW-SOL", "KRW-DOGE"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// 심볼 → 한국어 이름.
pub fn coin_name(symbol: &str) -> &'static str {
    match symbol.to_uppercase().as_str() {
        "BTC" => "비트코인",
        "ETH" => "이더리움",
        "XRP" => "리플",
        "SOL" => "솔라나",
        "DOGE" => "도지코인",
        "ADA" => "에이다",
        "TRX" => "트론",
        "USDT" => "테더",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_markets() {
        assert_eq!(coin_name("BTC"), "비트코인");
        assert_eq!(coin_name("btc"), "비트코인");
        assert_eq!(coin_name("ZZZ"), "");
        assert!(default_markets().contains(&"KRW-BTC".to_string()));
    }

    #[test]
    fn parse_tickers_valid() {
        let text = r#"[{"market":"KRW-BTC","trade_price":50000000.0,"signed_change_rate":0.012}]"#;
        let coins = parse_tickers(text, &["KRW-BTC".to_string()]).unwrap();
        assert_eq!(coins.len(), 1);
        assert_eq!(coins[0].symbol, "BTC");
        assert_eq!(coins[0].price, 50_000_000.0);
        assert!((coins[0].change_pct - 1.2).abs() < 1e-9);
    }

    #[test]
    fn parse_tickers_error_object_stays_clean() {
        // 업비트가 잘못된 심볼에 돌려주는 에러 객체 — serde 내부 오류가 새면 안 된다.
        let text = r#"{"error":{"name":"Code not found","message":"..."}}"#;
        let err = parse_tickers(text, &["KRW-ZZZZZZ".to_string()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("ZZZZZZ"), "심볼이 안내에 포함되어야: {err}");
        assert!(err.contains("코인을 찾을 수 없어요"));
        assert!(
            !err.to_lowercase().contains("invalid type"),
            "serde 내부 오류 노출 금지: {err}"
        );
        assert!(
            !err.contains("sequence"),
            "serde 내부 오류 노출 금지: {err}"
        );
    }

    #[test]
    fn parse_tickers_empty_array_stays_clean() {
        let err = parse_tickers("[]", &["KRW-NOPE".to_string()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("NOPE"));
        assert!(err.contains("코인을 찾을 수 없어요"));
    }

    #[tokio::test]
    #[ignore]
    async fn live_btc() {
        let c = fetch(&["KRW-BTC".to_string()]).await.unwrap();
        assert_eq!(c.len(), 1);
        assert!(c[0].price > 0.0);
        assert_eq!(c[0].symbol, "BTC");
    }
}

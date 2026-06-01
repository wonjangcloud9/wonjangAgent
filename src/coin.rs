//! 코인 시세 — 한국 1위 거래소 업비트의 실시간 시세(무료, 키 불필요).

use anyhow::{Context, Result};
use serde::Deserialize;

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
    let tickers: Vec<Ticker> = http
        .get(&url)
        .send()
        .await
        .context("코인 시세 요청 실패")?
        .json()
        .await
        .context("코인 시세 응답 파싱 실패(심볼을 확인하세요)")?;
    Ok(tickers
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
        .collect())
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

    #[tokio::test]
    #[ignore]
    async fn live_btc() {
        let c = fetch(&["KRW-BTC".to_string()]).await.unwrap();
        assert_eq!(c.len(), 1);
        assert!(c[0].price > 0.0);
        assert_eq!(c[0].symbol, "BTC");
    }
}

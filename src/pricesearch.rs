//! 가격비교 검색 런처 — 질의를 받아 한국 쇼핑 비교 사이트의 검색 URL을 만든다.
//!
//! 실시간 가격은 각 쇼핑몰 API 키/스크래핑이 필요해 검증할 수 없으므로(빌드 정책·
//! verify-or-don't-ship) 가져오지 않는다. 대신 한 번에 여러 가격비교 사이트를 열어주는
//! 런처를 제공한다 — URL 템플릿은 안정적이라 검증 가능하다(404 아님을 실측).

/// 검색 대상 사이트(이름, URL 템플릿 — `{q}`에 인코딩된 질의가 들어간다).
pub struct Site {
    pub name: &'static str,
    pub template: &'static str,
}

/// 상품 가격비교 사이트(가격비교 우선 순서).
pub const SHOPPING: &[Site] = &[
    Site {
        name: "네이버쇼핑(가격비교)",
        template: "https://search.shopping.naver.com/search/all?query={q}",
    },
    Site {
        name: "다나와",
        template: "https://search.danawa.com/dsearch.php?query={q}",
    },
    Site {
        name: "쿠팡",
        template: "https://www.coupang.com/np/search?q={q}",
    },
    Site {
        name: "구글쇼핑",
        template: "https://www.google.com/search?tbm=shop&q={q}",
    },
];

/// 질의를 URL 쿼리용으로 퍼센트 인코딩한다(UTF-8 바이트 단위, 한글 OK).
pub fn encode_query(s: &str) -> String {
    let mut out = String::new();
    for b in s.trim().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 사이트별 (이름, 완성된 검색 URL) 목록.
pub fn urls(sites: &[Site], query: &str) -> Vec<(&'static str, String)> {
    let q = encode_query(query);
    sites
        .iter()
        .map(|s| (s.name, s.template.replace("{q}", &q)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_handles_korean_and_space() {
        assert_eq!(encode_query("airpods"), "airpods");
        assert_eq!(encode_query("air pods"), "air%20pods");
        // 한글은 UTF-8 바이트마다 %XX (에어팟 = 9바이트).
        let e = encode_query("에어팟");
        assert!(e.starts_with('%') && e.matches('%').count() == 9);
        assert_eq!(encode_query("  trim  "), "trim");
    }

    #[test]
    fn urls_fill_template_for_all_sites() {
        let got = urls(SHOPPING, "에어팟 프로");
        assert_eq!(got.len(), SHOPPING.len());
        for (_, url) in &got {
            assert!(url.contains("%EC%97%90")); // '에' 인코딩 포함
            assert!(!url.contains("{q}")); // 템플릿 자리 모두 치환
            assert!(url.starts_with("https://"));
        }
        // 네이버쇼핑은 가격비교라 첫 번째.
        assert!(got[0].1.contains("shopping.naver.com"));
    }
}

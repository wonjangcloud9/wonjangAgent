//! 마진율 계산 — 원가·판매가로 이익/마진율/마크업을 내고, 목표 마진율로 판매가를 역산한다.
//!
//! 마진율(margin) = 이익 ÷ 판매가, 마크업(markup) = 이익 ÷ 원가. 둘을 혼동해
//! "30% 남기려면 원가에 30% 붙이면 된다"고 착각하는 흔한 오답을 바로잡는다
//! (목표 마진 판매가 = 원가 ÷ (1 - 마진율), 원가×1.3이 아니다).

/// (이익, 마진율%, 마크업%). 0 나눗셈은 0으로 방어한다(판매가·원가 0).
pub fn analyze(cost: f64, price: f64) -> (f64, f64, f64) {
    let profit = price - cost;
    let margin = if price != 0.0 {
        profit / price * 100.0
    } else {
        0.0
    };
    let markup = if cost != 0.0 {
        profit / cost * 100.0
    } else {
        0.0
    };
    (profit, margin, markup)
}

/// 목표 마진율(%)을 달성하는 판매가 = 원가 ÷ (1 - 마진율/100).
/// 마진율은 0 이상 100 미만만 가능(100%면 판매가 무한대) → 벗어나면 None.
pub fn price_for_margin(cost: f64, target_margin_pct: f64) -> Option<f64> {
    if !(0.0..100.0).contains(&target_margin_pct) {
        return None;
    }
    Some(cost / (1.0 - target_margin_pct / 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_margin_vs_markup() {
        // 원가 7000, 판매 10000 → 이익 3000, 마진율 30%, 마크업 42.857%.
        let (profit, margin, markup) = analyze(7000.0, 10000.0);
        assert_eq!(profit, 3000.0);
        assert!((margin - 30.0).abs() < 0.01);
        assert!((markup - 42.857).abs() < 0.01);
    }

    #[test]
    fn loss_is_negative_margin() {
        // 원가보다 싸게 팔면(손해) 마진율이 음수 — 경고 정보로 유용.
        let (profit, margin, _) = analyze(10000.0, 8000.0);
        assert_eq!(profit, -2000.0);
        assert!(margin < 0.0);
    }

    #[test]
    fn price_for_margin_is_divide_not_multiply() {
        // 마진 30% 판매가 = 7000 / 0.7 = 10000 (원가×1.3=9100이 아니다).
        let p = price_for_margin(7000.0, 30.0).unwrap();
        assert!((p - 10000.0).abs() < 0.01);
        assert!((p - 9100.0).abs() > 1.0); // 마크업 30%(원가×1.3)과 명확히 다름
                                           // 역으로 그 판매가의 마진율은 정확히 목표와 같다.
        let (_, margin, _) = analyze(7000.0, p);
        assert!((margin - 30.0).abs() < 0.01);
        // 100% 이상·음수는 불가.
        assert!(price_for_margin(7000.0, 100.0).is_none());
        assert!(price_for_margin(7000.0, -5.0).is_none());
    }
}

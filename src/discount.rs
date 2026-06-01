//! 할인가 계산 — 세일·쿠폰 등 할인 후 가격과 절약액을 계산한다.
//!
//! 중복 할인(예: 회원 20% + 카드 10%)은 순차 적용(곱셈)한다. 한국 쇼핑에서
//! 흔한 이중 할인을 그대로 반영한다. 순수 계산이라 키가 없다.

/// 할인 결과.
pub struct Discount {
    pub original: f64,       // 원가
    pub final_price: f64,    // 최종 할인가
    pub saved: f64,          // 절약액
    pub effective_rate: f64, // 실질 할인율(%)
}

/// 원가에 할인율(%)들을 순차 적용한다. 빈 목록이면 할인 없음.
pub fn apply(original: f64, rates: &[f64]) -> Discount {
    let mut price = original;
    for r in rates {
        let r = r.clamp(0.0, 100.0);
        price *= 1.0 - r / 100.0;
    }
    let saved = original - price;
    let effective_rate = if original > 0.0 {
        saved / original * 100.0
    } else {
        0.0
    };
    Discount {
        original,
        final_price: price,
        saved,
        effective_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_discount() {
        // 30,000원 20% → 24,000원, 6,000원 절약.
        let d = apply(30_000.0, &[20.0]);
        assert!((d.final_price - 24_000.0).abs() < 0.01);
        assert!((d.saved - 6_000.0).abs() < 0.01);
        assert!((d.effective_rate - 20.0).abs() < 0.01);
    }

    #[test]
    fn double_discount_is_multiplicative() {
        // 10,000원 20% 후 10% → 10000×0.8×0.9 = 7,200원(실질 28%).
        let d = apply(10_000.0, &[20.0, 10.0]);
        assert!((d.final_price - 7_200.0).abs() < 0.01);
        assert!((d.effective_rate - 28.0).abs() < 0.01);
    }

    #[test]
    fn no_discount() {
        let d = apply(5_000.0, &[]);
        assert_eq!(d.final_price, 5_000.0);
        assert_eq!(d.saved, 0.0);
    }

    #[test]
    fn rate_is_clamped() {
        // 120%는 100%로 클램프 → 전액 할인.
        let d = apply(1_000.0, &[120.0]);
        assert!((d.final_price - 0.0).abs() < 0.01);
    }
}

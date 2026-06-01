//! 평수 변환 — 한국 부동산에서 매일 쓰는 평 ↔ 제곱미터(㎡).
//!
//! 1평 = 400/121 ㎡ ≈ 3.305785㎡.

/// 1평의 제곱미터 값.
pub const M2_PER_PYEONG: f64 = 400.0 / 121.0;

/// 평 → 제곱미터.
pub fn pyeong_to_m2(p: f64) -> f64 {
    p * M2_PER_PYEONG
}

/// 제곱미터 → 평.
pub fn m2_to_pyeong(m: f64) -> f64 {
    m / M2_PER_PYEONG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        // 30평 ≈ 99.17㎡
        assert!((pyeong_to_m2(30.0) - 99.17).abs() < 0.1);
        // 84㎡(국민주택) ≈ 25.4평
        assert!((m2_to_pyeong(84.0) - 25.41).abs() < 0.1);
        // 왕복 정합성
        assert!((m2_to_pyeong(pyeong_to_m2(25.0)) - 25.0).abs() < 1e-9);
    }
}

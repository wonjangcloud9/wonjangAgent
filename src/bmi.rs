//! BMI(체질량지수) 계산 — 대한비만학회 아시아-태평양 기준으로 판정.
//!
//! 서양 기준(정상 18.5~24.9)과 달리 한국·아시아는 비만 기준이 더 엄격하다
//! (정상 18.5~22.9, 과체중 23~24.9, 비만 25 이상). 순수 계산이라 키가 없다.

/// BMI 결과.
pub struct Bmi {
    pub value: f64,          // BMI 값
    pub grade: &'static str, // 판정(아시아 기준)
    pub standard_kg: f64,    // 표준체중(BMI 22 기준)
}

/// 판정 등급(대한비만학회 아시아-태평양 기준).
fn grade_of(bmi: f64) -> &'static str {
    match bmi {
        b if b < 18.5 => "저체중",
        b if b < 23.0 => "정상",
        b if b < 25.0 => "과체중(비만 전단계)",
        b if b < 30.0 => "1단계 비만",
        b if b < 35.0 => "2단계 비만",
        _ => "3단계 비만(고도비만)",
    }
}

/// 키(cm)와 몸무게(kg)로 BMI를 계산한다. 키가 0 이하이면 None.
pub fn calc(height_cm: f64, weight_kg: f64) -> Option<Bmi> {
    if height_cm <= 0.0 || weight_kg <= 0.0 {
        return None;
    }
    let h = height_cm / 100.0;
    let value = weight_kg / (h * h);
    Some(Bmi {
        value,
        grade: grade_of(value),
        standard_kg: 22.0 * h * h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_range() {
        // 175cm 68kg → BMI 22.2, 정상.
        let b = calc(175.0, 68.0).unwrap();
        assert!((b.value - 22.2).abs() < 0.1);
        assert_eq!(b.grade, "정상");
    }

    #[test]
    fn asian_overweight_cutoff() {
        // BMI 24는 아시아 기준 과체중(서양은 정상).
        assert_eq!(grade_of(24.0), "과체중(비만 전단계)");
        // BMI 25는 비만.
        assert_eq!(grade_of(25.0), "1단계 비만");
    }

    #[test]
    fn standard_weight_for_height() {
        // 170cm 표준체중 = 22 × 1.7² = 63.58kg.
        let b = calc(170.0, 70.0).unwrap();
        assert!((b.standard_kg - 63.58).abs() < 0.1);
    }

    #[test]
    fn invalid_inputs() {
        assert!(calc(0.0, 60.0).is_none());
        assert!(calc(170.0, 0.0).is_none());
    }
}

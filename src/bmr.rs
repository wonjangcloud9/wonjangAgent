//! 기초대사량(BMR)·하루 권장 칼로리 — 다이어트·운동 계획에 쓴다.
//!
//! 미플린-세인트 지어(Mifflin-St Jeor) 공식으로 BMR을 구하고, 활동 수준별
//! 하루 필요 칼로리(TDEE)를 함께 보여준다. 순수 계산이라 키가 없다.

use anyhow::{anyhow, Result};

/// 성별.
#[derive(Clone, Copy)]
pub enum Sex {
    Male,
    Female,
}

impl Sex {
    pub fn parse(s: &str) -> Result<Sex> {
        match s.trim().to_lowercase().as_str() {
            "남" | "남자" | "m" | "male" => Ok(Sex::Male),
            "여" | "여자" | "f" | "female" => Ok(Sex::Female),
            other => Err(anyhow!("성별은 남/여로 입력하세요 (입력: {other})")),
        }
    }
}

/// 기초대사량(kcal/일). 미플린-세인트 지어 공식.
pub fn bmr(sex: Sex, age: u32, height_cm: f64, weight_kg: f64) -> f64 {
    let base = 10.0 * weight_kg + 6.25 * height_cm - 5.0 * age as f64;
    match sex {
        Sex::Male => base + 5.0,
        Sex::Female => base - 161.0,
    }
}

/// 활동 수준별 (이름, 계수, 설명).
pub const ACTIVITY: &[(&str, f64, &str)] = &[
    ("좌식", 1.2, "운동 거의 안 함"),
    ("가벼움", 1.375, "주 1~3회 가벼운 운동"),
    ("보통", 1.55, "주 3~5회 운동"),
    ("활발", 1.725, "주 6~7회 운동"),
    ("매우 활발", 1.9, "하루 2회·육체노동"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn male_bmr() {
        // 남 30세 175cm 70kg → 10*70+6.25*175-5*30+5 = 700+1093.75-150+5 = 1648.75.
        let b = bmr(Sex::Male, 30, 175.0, 70.0);
        assert!((b - 1648.75).abs() < 0.01);
    }

    #[test]
    fn female_bmr() {
        // 여 30세 160cm 55kg → 550+1000-150-161 = 1239.
        let b = bmr(Sex::Female, 30, 160.0, 55.0);
        assert!((b - 1239.0).abs() < 0.01);
    }

    #[test]
    fn sex_parse() {
        assert!(matches!(Sex::parse("남").unwrap(), Sex::Male));
        assert!(matches!(Sex::parse("female").unwrap(), Sex::Female));
        assert!(Sex::parse("x").is_err());
    }
}

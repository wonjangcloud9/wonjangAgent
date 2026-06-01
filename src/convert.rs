//! 단위 변환 — 온도·무게·길이 등 일상에서 자주 쓰는 변환.
//!
//! 입력 단위에 따라 대응하는 단위로 환산한다. 외부 의존성·키가 없다.

/// 변환 결과(설명 문구 + 값).
pub struct Conversion {
    pub label: String, // 예: "100℃ = 212℉"
}

/// 값과 단위 문자열을 받아 변환 결과를 만든다. 모르는 단위면 None.
pub fn convert(value: f64, unit: &str) -> Option<Conversion> {
    let u = unit.trim().to_lowercase();
    let label = match u.as_str() {
        // 온도.
        "c" | "섭씨" | "℃" => {
            let f = value * 9.0 / 5.0 + 32.0;
            format!("{value:.1}℃ = {f:.1}℉")
        }
        "f" | "화씨" | "℉" => {
            let c = (value - 32.0) * 5.0 / 9.0;
            format!("{value:.1}℉ = {c:.1}℃")
        }
        // 무게.
        "kg" | "킬로" | "킬로그램" => {
            let lb = value * 2.204_622_6;
            format!("{value:.2}kg = {lb:.2}lb")
        }
        "lb" | "파운드" => {
            let kg = value / 2.204_622_6;
            format!("{value:.2}lb = {kg:.2}kg")
        }
        // 길이.
        "cm" | "센티" | "센티미터" => {
            let inch = value / 2.54;
            format!("{value:.1}cm = {inch:.2}inch")
        }
        "inch" | "in" | "인치" => {
            let cm = value * 2.54;
            format!("{value:.2}inch = {cm:.1}cm")
        }
        "km" | "킬로미터" => {
            let mile = value / 1.609_344;
            format!("{value:.2}km = {mile:.2}mile")
        }
        "mile" | "mi" | "마일" => {
            let km = value * 1.609_344;
            format!("{value:.2}mile = {km:.2}km")
        }
        _ => return None,
    };
    Some(Conversion { label })
}

/// 지원하는 입력 단위 안내.
pub fn supported() -> &'static str {
    "c/f(온도) · kg/lb(무게) · cm/inch · km/mile(길이)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celsius_to_fahrenheit() {
        assert_eq!(convert(100.0, "c").unwrap().label, "100.0℃ = 212.0℉");
        assert_eq!(convert(0.0, "섭씨").unwrap().label, "0.0℃ = 32.0℉");
    }

    #[test]
    fn fahrenheit_to_celsius() {
        assert_eq!(convert(32.0, "f").unwrap().label, "32.0℉ = 0.0℃");
    }

    #[test]
    fn weight_and_length() {
        assert!(convert(1.0, "kg").unwrap().label.contains("2.20lb"));
        assert!(convert(2.54, "cm").unwrap().label.contains("1.00inch"));
        assert!(convert(1.0, "mile").unwrap().label.contains("1.61km"));
    }

    #[test]
    fn unknown_unit() {
        assert!(convert(1.0, "광년").is_none());
    }
}

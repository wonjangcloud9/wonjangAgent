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
        // 속도.
        "kmh" | "kph" | "시속" => {
            let mph = value / 1.609_344;
            format!("{value:.1}km/h = {mph:.1}mph")
        }
        "mph" => {
            let kmh = value * 1.609_344;
            format!("{value:.1}mph = {kmh:.1}km/h")
        }
        // 부피(US 갤런).
        "l" | "리터" => {
            let gal = value / 3.785_411_784;
            format!("{value:.2}L = {gal:.2}gal")
        }
        "gal" | "갤런" => {
            let l = value * 3.785_411_784;
            format!("{value:.2}gal = {l:.2}L")
        }
        // 넓이(제곱미터↔제곱피트).
        "sqm" | "㎡" | "제곱미터" => {
            let sqft = value * 10.763_910_4;
            format!("{value:.2}㎡ = {sqft:.2}ft²")
        }
        "sqft" | "ft2" => {
            let sqm = value / 10.763_910_4;
            format!("{value:.2}ft² = {sqm:.2}㎡")
        }
        // 한국 전통 단위(무게) — 금은방·정육점에서 매일 쓰는데 GPT는 근=600g/375g을 자주 헷갈린다.
        "돈" => {
            // 금·은 무게. 1돈 = 3.75g.
            let g = value * 3.75;
            format!("{value}돈 = {g:.2}g  (금·은 기준, 1돈=3.75g)")
        }
        "근" => {
            // 고기·채소. 1근 = 600g(정육점 표준).
            let g = value * 600.0;
            if g >= 1000.0 {
                format!(
                    "{value}근 = {:.2}kg ({g:.0}g)  (고기·채소 기준, 1근=600g)",
                    g / 1000.0
                )
            } else {
                format!("{value}근 = {g:.0}g  (고기·채소 기준, 1근=600g)")
            }
        }
        "관" => {
            // 1관 = 3.75kg = 1000돈. 도매·농수산물.
            let kg = value * 3.75;
            format!("{value}관 = {kg:.2}kg  (1관=3.75kg=1000돈)")
        }
        "그램" | "g" => {
            // 그램 → 돈·근 역환산(금 시세·고기 살 때 둘 다 자주 묻는다).
            let don = value / 3.75;
            let geun = value / 600.0;
            format!("{value}g = {don:.2}돈 = {geun:.3}근")
        }
        // 한국 전통 단위(부피) — 쌀·곡식.
        "되" => {
            let l = value * 1.8;
            format!("{value}되 = {l:.2}L  (쌀·곡식, 1되≈1.8L)")
        }
        "말" => {
            let l = value * 18.0;
            format!("{value}말 = {l:.1}L  (1말=10되=18L)")
        }
        // 한국 전통 단위(길이) — 옷감·이불·커튼·한복에서 쓴다("킹 이불 7자").
        "자" => {
            let cm = value * 30.3;
            format!(
                "{value}자 = {cm:.1}cm ({:.2}m)  (옷감·한복 기준, 1자=30.3cm)",
                cm / 100.0
            )
        }
        "치" => {
            let cm = value * 3.03;
            format!("{value}치 = {cm:.2}cm  (1자=10치, 1치=3.03cm)")
        }
        _ => return None,
    };
    Some(Conversion { label })
}

/// 지원하는 입력 단위 안내.
pub fn supported() -> &'static str {
    "c/f(온도) · kg/lb(무게) · cm/inch · km/mile(길이) · kmh/mph(속도) · l/gal(부피) · sqm/sqft(넓이) · 돈/근/관·g·되/말·자/치(한국 단위)"
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

    #[test]
    fn speed_volume_area() {
        assert!(convert(100.0, "kmh").unwrap().label.contains("62.1mph"));
        assert!(convert(1.0, "gal").unwrap().label.contains("3.79L"));
        assert!(convert(1.0, "sqm").unwrap().label.contains("10.76ft²"));
    }

    #[test]
    fn korean_traditional_weight() {
        // 금 5돈 = 18.75g(금은방).
        assert!(convert(5.0, "돈").unwrap().label.contains("18.75g"));
        // 고기 2근 = 1.20kg(정육점, 600g 기준).
        let geun = convert(2.0, "근").unwrap().label;
        assert!(geun.contains("1.20kg") && geun.contains("600g"), "{geun}");
        // 1근(=600g)은 kg 미만이라 g로.
        assert!(convert(1.0, "근").unwrap().label.contains("600g"));
        // 1관 = 3.75kg.
        assert!(convert(1.0, "관").unwrap().label.contains("3.75kg"));
        // 그램 역환산: 600g = 1근.
        let g = convert(600.0, "g").unwrap().label;
        assert!(g.contains("160.00돈") && g.contains("1.000근"), "{g}");
    }

    #[test]
    fn korean_traditional_volume() {
        assert!(convert(1.0, "되").unwrap().label.contains("1.80L"));
        assert!(convert(1.0, "말").unwrap().label.contains("18.0L"));
    }

    #[test]
    fn korean_traditional_length() {
        // 옷감·이불: 7자 = 212.1cm.
        assert!(convert(7.0, "자").unwrap().label.contains("212.1cm"));
        assert!(convert(1.0, "자").unwrap().label.contains("30.3cm"));
        assert!(convert(1.0, "치").unwrap().label.contains("3.03cm"));
    }
}

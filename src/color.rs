//! 색상 변환 — HEX ↔ RGB ↔ HSL. 디자인·CSS 작업에 자주 쓴다.
//!
//! `#ff5733` 같은 헥스나 `255 87 51` 같은 RGB를 받아 세 표기를 모두 보여준다.
//! 외부 의존성·키가 없다.

use anyhow::{anyhow, Result};

/// RGB 색(0~255).
#[derive(Clone, Copy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// 헥스 문자열(#rgb, #rrggbb, 또는 # 없이)을 파싱한다.
pub fn parse_hex(s: &str) -> Result<Rgb> {
    let h = s.trim().trim_start_matches('#');
    let expanded = if h.len() == 3 {
        // #rgb → #rrggbb.
        h.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        h.to_string()
    };
    if expanded.len() != 6 || !expanded.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!("헥스 색상 형식: #ff5733 또는 #f53"));
    }
    let v = |i: usize| u8::from_str_radix(&expanded[i..i + 2], 16).unwrap();
    Ok(Rgb {
        r: v(0),
        g: v(2),
        b: v(4),
    })
}

/// RGB를 헥스 문자열로.
pub fn to_hex(c: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// RGB → HSL(H 0~360°, S·L 0~100%).
pub fn to_hsl(c: Rgb) -> (f64, f64, f64) {
    let r = c.r as f64 / 255.0;
    let g = c.g as f64 / 255.0;
    let b = c.b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < f64::EPSILON {
        return (0.0, 0.0, l * 100.0);
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f64::EPSILON {
        ((g - b) / d) % 6.0
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    let mut h = h * 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    (h, s * 100.0, l * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_format_hex() {
        let c = parse_hex("#ff5733").unwrap();
        assert_eq!((c.r, c.g, c.b), (255, 87, 51));
        assert_eq!(to_hex(c), "#FF5733");
        // 짧은 형식.
        let c2 = parse_hex("#f53").unwrap();
        assert_eq!((c2.r, c2.g, c2.b), (255, 85, 51));
        // # 없이.
        assert!(parse_hex("00ff00").is_ok());
    }

    #[test]
    fn hsl_known_values() {
        // 빨강.
        let (h, s, l) = to_hsl(Rgb { r: 255, g: 0, b: 0 });
        assert!((h - 0.0).abs() < 0.5);
        assert!((s - 100.0).abs() < 0.5);
        assert!((l - 50.0).abs() < 0.5);
        // 흰색.
        let (_, s2, l2) = to_hsl(Rgb {
            r: 255,
            g: 255,
            b: 255,
        });
        assert!((s2 - 0.0).abs() < 0.5);
        assert!((l2 - 100.0).abs() < 0.5);
    }

    #[test]
    fn rejects_bad_hex() {
        assert!(parse_hex("#zzz").is_err());
        assert!(parse_hex("#12").is_err());
    }
}

//! 세계 시간 — 주요 도시의 현재 시각을 DST까지 정확히 보여준다.
//!
//! "지금 뉴욕 몇 시?", "한국 오전 9시는 LA 몇 시?" 같은 해외 업무·연락에 쓴다.
//! chrono-tz의 내장 IANA tz 데이터로 서머타임을 정확히 반영한다(키 불필요).

use chrono::Utc;
use chrono_tz::Tz;

/// (한국어 도시명, IANA 타임존). 주요 도시 위주.
const CITIES: &[(&str, Tz)] = &[
    ("서울", Tz::Asia__Seoul),
    ("도쿄", Tz::Asia__Tokyo),
    ("베이징", Tz::Asia__Shanghai),
    ("싱가포르", Tz::Asia__Singapore),
    ("방콕", Tz::Asia__Bangkok),
    ("두바이", Tz::Asia__Dubai),
    ("런던", Tz::Europe__London),
    ("파리", Tz::Europe__Paris),
    ("뉴욕", Tz::America__New_York),
    ("LA", Tz::America__Los_Angeles),
    ("시드니", Tz::Australia__Sydney),
    ("하와이", Tz::Pacific__Honolulu),
];

/// 한 도시의 현재 시각 정보.
pub struct CityTime {
    pub name: String,
    pub time: String,   // "MM-DD HH:MM (요일)"
    pub offset: String, // "UTC+9"
}

fn weekday_kr(d: chrono::Weekday) -> &'static str {
    use chrono::Weekday::*;
    match d {
        Mon => "월",
        Tue => "화",
        Wed => "수",
        Thu => "목",
        Fri => "금",
        Sat => "토",
        Sun => "일",
    }
}

fn format_city(name: &str, tz: Tz) -> CityTime {
    use chrono::{Datelike, Offset};
    let now = Utc::now().with_timezone(&tz);
    let off_sec = now.offset().fix().local_minus_utc();
    let off_h = off_sec / 3600;
    let off_m = (off_sec.abs() % 3600) / 60;
    let offset = if off_m == 0 {
        format!("UTC{off_h:+}")
    } else {
        format!("UTC{off_h:+}:{off_m:02}")
    };
    CityTime {
        name: name.to_string(),
        time: format!(
            "{} ({})",
            now.format("%m-%d %H:%M"),
            weekday_kr(now.weekday())
        ),
        offset,
    }
}

/// 검색어로 도시를 찾는다(부분일치). 비면 전체.
pub fn lookup(query: Option<&str>) -> Vec<CityTime> {
    match query {
        Some(q) => {
            let q = q.trim().to_lowercase();
            CITIES
                .iter()
                .filter(|(name, _)| name.to_lowercase().contains(&q))
                .map(|(name, tz)| format_city(name, *tz))
                .collect()
        }
        None => CITIES
            .iter()
            .map(|(name, tz)| format_city(name, *tz))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_all_cities() {
        let all = lookup(None);
        assert_eq!(all.len(), CITIES.len());
        assert!(all.iter().any(|c| c.name == "서울"));
    }

    #[test]
    fn seoul_offset_is_utc9() {
        let seoul = lookup(Some("서울"));
        assert_eq!(seoul.len(), 1);
        assert_eq!(seoul[0].offset, "UTC+9");
    }

    #[test]
    fn partial_search() {
        assert_eq!(lookup(Some("뉴욕")).len(), 1);
        assert!(lookup(Some("없는도시")).is_empty());
    }
}

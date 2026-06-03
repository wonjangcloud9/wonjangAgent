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

/// 이름(부분일치)으로 도시의 (정식명, 타임존)을 찾는다.
pub fn find_tz(name: &str) -> Option<(&'static str, Tz)> {
    let q = name.trim().to_lowercase();
    // 빈 질의는 모든 도시에 매칭("".contains는 항상 true)되어 첫 도시로 조용히 빠지므로 차단.
    if q.is_empty() {
        return None;
    }
    CITIES
        .iter()
        .find(|(n, _)| n.to_lowercase().contains(&q))
        .map(|(n, tz)| (*n, *tz))
}

/// 도시 간 시간 변환 결과.
pub struct Conversion {
    pub from_label: String, // "서울 09:00 (화)"
    pub to_label: String,   // "뉴욕 20:00 (월)"
    pub day_note: String,   // "" / "(전날)" / "(다음날)"
}

/// "HH:MM"을 from 도시 오늘 날짜 기준으로 to 도시 시각으로 변환한다.
pub fn convert(time: &str, from: &str, to: &str) -> anyhow::Result<Conversion> {
    use chrono::{Datelike, NaiveTime, TimeZone};
    let (from_name, from_tz) = find_tz(from)
        .ok_or_else(|| anyhow::anyhow!("출발 도시를 못 찾았어요: {from} (서울/뉴욕/런던 등)"))?;
    let (to_name, to_tz) =
        find_tz(to).ok_or_else(|| anyhow::anyhow!("도착 도시를 못 찾았어요: {to}"))?;
    let t = NaiveTime::parse_from_str(time.trim(), "%H:%M")
        .map_err(|_| anyhow::anyhow!("시각은 HH:MM 형식으로 입력하세요 (예: 09:00)"))?;

    // from 도시의 '오늘' 날짜 + 입력 시각.
    let from_today = Utc::now().with_timezone(&from_tz).date_naive();
    let naive = from_today.and_time(t);
    let from_dt = from_tz
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| anyhow::anyhow!("해당 시각 변환 실패(서머타임 경계일 수 있어요)"))?;
    let to_dt = from_dt.with_timezone(&to_tz);

    let day_diff = to_dt.date_naive().num_days_from_ce() - from_dt.date_naive().num_days_from_ce();
    let day_note = match day_diff {
        d if d < 0 => "(전날)".to_string(),
        0 => String::new(),
        _ => "(다음날)".to_string(),
    };
    Ok(Conversion {
        from_label: format!(
            "{from_name} {} ({})",
            from_dt.format("%H:%M"),
            weekday_kr(from_dt.weekday())
        ),
        to_label: format!(
            "{to_name} {} ({})",
            to_dt.format("%H:%M"),
            weekday_kr(to_dt.weekday())
        ),
        day_note,
    })
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

    #[test]
    fn convert_seoul_to_newyork_is_earlier() {
        // 서울→뉴욕은 13~14시간 느림(전날이 되기 쉬움). 변환이 성공하고 라벨이 채워짐.
        let c = convert("09:00", "서울", "뉴욕").unwrap();
        assert!(c.from_label.starts_with("서울 09:00"));
        assert!(c.to_label.starts_with("뉴욕"));
    }

    #[test]
    fn convert_rejects_unknown_city() {
        assert!(convert("09:00", "화성", "뉴욕").is_err());
        assert!(convert("bad", "서울", "뉴욕").is_err());
    }
}

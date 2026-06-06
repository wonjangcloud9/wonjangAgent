//! 디데이(D-day) 관리 — 한국 사용자가 사랑하는 디데이 문화.
//!
//! 수능·기념일·마감 등 중요한 날까지 남은 일수를 챙기고 아침 브리핑에 보여준다.
//! 날짜 계산은 chrono(로컬 타임존)로 한다.
//!
//! 저장 위치: `~/.local/share/wonjang/ddays.json`

use anyhow::{bail, Context, Result};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dday {
    pub id: u64,
    pub label: String,
    /// 목표 날짜(YYYY-MM-DD). 매년 반복이면 '첫' 발생일(연도는 참고용).
    pub date: String,
    /// 매년 반복(생신·결혼기념일 등) — 지나도 다음 해로 굴러간다.
    #[serde(default)]
    pub annual: bool,
}

/// 디데이의 '유효 날짜'. 매년 반복이면 오늘(포함) 이후 가장 가까운 그 월/일,
/// 아니면 저장된 날짜 그대로. 2/29는 평년이면 2/28로 클램프.
pub fn effective_date(base: NaiveDate, annual: bool, today: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    if !annual {
        return base;
    }
    let (m, day) = (base.month(), base.day());
    let this = clamp_md(today.year(), m, day);
    if this >= today {
        this
    } else {
        clamp_md(today.year() + 1, m, day)
    }
}

fn clamp_md(year: i32, m: u32, day: u32) -> NaiveDate {
    use chrono::Datelike;
    NaiveDate::from_ymd_opt(year, m, day)
        .or_else(|| NaiveDate::from_ymd_opt(year, m, 28)) // 2/29 평년 → 2/28
        .unwrap_or_else(|| today().with_day(1).unwrap())
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DdayStore {
    #[serde(default)]
    pub items: Vec<Dday>,
    #[serde(default)]
    next_id: u64,
}

fn store_path() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("데이터 디렉터리를 찾을 수 없습니다")?
        .join("wonjang");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("ddays.json"))
}

/// 오늘 날짜(로컬).
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// 날짜 문자열을 파싱(YYYY-MM-DD). 흔한 한국식 표기(`.`·`/`·8자리)도 허용.
pub fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(&crate::datecalc::normalize_date(s), "%Y-%m-%d")
        .with_context(|| format!("날짜 형식이 올바르지 않습니다(YYYY-MM-DD): '{s}'"))
}

/// 목표 날짜까지 남은 일수(오늘 기준). 음수면 지난 날.
pub fn days_until(date: NaiveDate, today: NaiveDate) -> i64 {
    (date - today).num_days()
}

/// 남은 일수를 D-라벨로(D-DAY / D-30 / D+5).
pub fn dday_label(days: i64) -> String {
    match days {
        0 => "D-DAY".to_string(),
        d if d > 0 => format!("D-{d}"),
        d => format!("D+{}", -d),
    }
}

/// iCalendar 텍스트값 이스케이프(RFC 5545: `\` `;` `,` 줄바꿈).
fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// 디데이들을 iCalendar(.ics) 문자열로 — 구글·애플 캘린더에 '가져오기'로 넣는다.
/// 각 디데이는 해당 날짜의 종일(all-day) 일정이 된다. `dtstamp`는 호출부에서 현재 UTC로.
pub fn to_ics(items: &[Dday], dtstamp: &str) -> String {
    let mut out = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//wonjang//dday//KO\r\nCALSCALE:GREGORIAN\r\n",
    );
    for d in items {
        let date = match NaiveDate::parse_from_str(&d.date, "%Y-%m-%d") {
            Ok(dt) => dt,
            Err(_) => continue, // 날짜 형식이 깨진 항목은 건너뜀
        };
        let start = date.format("%Y%m%d").to_string();
        // 종일 일정의 DTEND는 다음 날(배타적, RFC 5545).
        let end = (date + chrono::Duration::days(1))
            .format("%Y%m%d")
            .to_string();
        out.push_str("BEGIN:VEVENT\r\n");
        out.push_str(&format!("UID:dday-{}@wonjang\r\n", d.id));
        out.push_str(&format!("DTSTAMP:{dtstamp}\r\n"));
        out.push_str(&format!("DTSTART;VALUE=DATE:{start}\r\n"));
        out.push_str(&format!("DTEND;VALUE=DATE:{end}\r\n"));
        out.push_str(&format!("SUMMARY:{}\r\n", ics_escape(&d.label)));
        out.push_str("END:VEVENT\r\n");
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

impl DdayStore {
    pub fn load() -> Result<Self> {
        let path = store_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(crate::util::load_json_or_recover(&path))
    }

    pub fn save(&self) -> Result<()> {
        crate::util::atomic_write(
            &store_path()?,
            serde_json::to_string_pretty(self)?.as_bytes(),
        )?;
        Ok(())
    }

    /// 디데이를 추가한다(날짜 유효성 검증). `annual`이면 매년 반복.
    pub fn add(&mut self, label: &str, date: &str, annual: bool) -> Result<u64> {
        let label = label.trim();
        if label.is_empty() {
            bail!("디데이 이름이 필요합니다");
        }
        let parsed = parse_date(date)?;
        let id = self
            .items
            .iter()
            .map(|x| x.id)
            .max()
            .unwrap_or(0)
            .max(self.next_id)
            .saturating_add(1);
        self.next_id = id;
        self.items.push(Dday {
            id,
            label: label.to_string(),
            date: parsed.format("%Y-%m-%d").to_string(),
            annual,
        });
        self.sort();
        self.save()?;
        Ok(id)
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|d| d.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// 날짜순 정렬(가까운 미래가 위로 오도록 지난 날은 뒤).
    fn sort(&mut self) {
        let today = today();
        self.items.sort_by_key(|d| {
            let days = parse_date(&d.date)
                .map(|dt| days_until(effective_date(dt, d.annual, today), today))
                .unwrap_or(i64::MAX);
            // 미래(>=0)를 먼저, 그다음 과거. 같은 부호면 가까운 순.
            if days >= 0 {
                (0, days)
            } else {
                (1, -days)
            }
        });
    }

    pub fn all(&self) -> &[Dday] {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_labels() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        assert_eq!(days_until(target, today), 30);
        assert_eq!(dday_label(30), "D-30");
        assert_eq!(dday_label(0), "D-DAY");
        assert_eq!(dday_label(-5), "D+5");
    }

    #[test]
    fn to_ics_makes_allday_events_and_escapes() {
        let items = vec![
            Dday {
                id: 1,
                label: "수능".into(),
                date: "2026-11-19".into(),
                annual: false,
            },
            Dday {
                id: 2,
                label: "회의, 발표".into(), // 콤마 이스케이프
                date: "2026-07-15".into(),
                annual: false,
            },
            Dday {
                id: 3,
                label: "깨진날짜".into(),
                date: "엉망".into(), // 건너뜀
                annual: false,
            },
        ];
        let ics = to_ics(&items, "20260604T000000Z");
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(ics.trim_end().ends_with("END:VCALENDAR"));
        assert!(ics.contains("DTSTART;VALUE=DATE:20261119"));
        assert!(ics.contains("DTEND;VALUE=DATE:20261120")); // +1일(배타적)
        assert!(ics.contains("SUMMARY:수능"));
        assert!(ics.contains("SUMMARY:회의\\, 발표")); // 콤마 이스케이프
        assert!(ics.contains("UID:dday-1@wonjang"));
        // 깨진 날짜는 이벤트 생성 안 함 → VEVENT 2개.
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 2);
    }

    #[test]
    fn parse_validates() {
        assert!(parse_date("2026-12-25").is_ok());
        // 흔한 한국식 표기도 허용(정규화).
        assert_eq!(
            parse_date("2026.12.25").unwrap(),
            parse_date("2026-12-25").unwrap()
        );
        assert_eq!(
            parse_date("2026/12/25").unwrap(),
            parse_date("2026-12-25").unwrap()
        );
        assert_eq!(
            parse_date("20261225").unwrap(),
            parse_date("2026-12-25").unwrap()
        );
        assert_eq!(
            parse_date("2026. 12. 25.").unwrap(),
            parse_date("2026-12-25").unwrap()
        );
        // 진짜 엉뚱한 입력은 여전히 에러.
        assert!(parse_date("아무거나").is_err());
        assert!(parse_date("2026-13-40").is_err());
    }

    #[test]
    fn add_sorts_future_first() {
        let mut s = DdayStore::default();
        // save를 피하려 직접 구성 후 로직 확인은 days_until로.
        s.items.push(Dday {
            id: 1,
            label: "지난거".into(),
            date: "2000-01-01".into(),
            annual: false,
        });
        s.items.push(Dday {
            id: 2,
            label: "미래".into(),
            date: "2999-01-01".into(),
            annual: false,
        });
        s.sort();
        assert_eq!(s.items[0].label, "미래"); // 미래가 먼저

        // 매년 반복: 지난 월/일이어도 다음 발생으로 굴러가 '지남'이 안 된다.
        let today = NaiveDate::from_ymd_opt(2026, 6, 6).unwrap();
        let base = parse_date("2000-05-20").unwrap(); // 과거지만 매년
        let eff = effective_date(base, true, today);
        assert!(eff >= today); // 다음 발생(2027-05-20)
        assert_eq!(eff, NaiveDate::from_ymd_opt(2027, 5, 20).unwrap());
        // 비반복은 그대로.
        assert_eq!(effective_date(base, false, today), base);
        // 2/29 매년 → 평년(2027)이면 2/28로.
        let leap = parse_date("2024-02-29").unwrap();
        assert_eq!(
            effective_date(leap, true, NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
            NaiveDate::from_ymd_opt(2027, 2, 28).unwrap()
        );
    }
}

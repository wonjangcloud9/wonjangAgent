//! 로컬 표 파일(CSV·엑셀) 읽기 — 내 컴퓨터의 파일을 직접 다루는 비서 기능.
//!
//! GPT처럼 "물어보는" 게 아니라 실제 파일을 열어 행·열을 요약하고 특정 열의
//! 합계·평균을 낸다. CSV는 내장 파서, 엑셀(.xlsx 등)은 calamine으로 읽는다.

use anyhow::{anyhow, Result};
use std::path::Path;

/// 표 데이터(머리글 + 행들).
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// 확장자에 따라 CSV 또는 엑셀(첫 시트)로 읽는다.
    pub fn load(path: &str) -> Result<Table> {
        Self::load_sheet(path, None)
    }

    /// 엑셀이면 지정 시트(이름 또는 1-기반 번호; 생략 시 첫 시트)를, CSV면 그대로 읽는다.
    pub fn load_sheet(path: &str, sheet: Option<&str>) -> Result<Table> {
        if !Path::new(path).exists() {
            return Err(anyhow!("파일을 찾을 수 없어요: {path}"));
        }
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "xlsx" | "xls" | "xlsm" | "xlsb" | "ods" => load_spreadsheet(path, sheet),
            _ => load_csv(path), // CSV는 시트 개념이 없어 sheet 무시
        }
    }

    /// 열 이름(대소문자 무시) 또는 1-기반 번호로 열 인덱스를 찾는다.
    pub fn col_index(&self, key: &str) -> Option<usize> {
        let key = key.trim();
        if let Some(i) = self
            .headers
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(key))
        {
            return Some(i);
        }
        if let Ok(n) = key.parse::<usize>() {
            if n >= 1 && n <= self.headers.len() {
                return Some(n - 1);
            }
        }
        None
    }

    /// 한 열에서 숫자로 해석되는 값들만 모은다.
    pub fn numeric_column(&self, idx: usize) -> Vec<f64> {
        self.rows
            .iter()
            .filter_map(|r| r.get(idx))
            .filter_map(|s| parse_num(s))
            .collect()
    }

    /// `group_idx` 열 값으로 묶어, `value_idx` 열의 합계·숫자개수·행수를 낸다.
    /// 입력에서 처음 본 순서를 기억하되, 결과는 합계 내림차순으로 정렬해 돌려준다.
    /// (피벗/그룹바이 — "지점별 매출 합계" 같은 가장 흔한 표 분석)
    pub fn group_by(&self, group_idx: usize, value_idx: usize) -> Vec<GroupStat> {
        use std::collections::HashMap;
        let mut order: Vec<String> = Vec::new();
        let mut map: HashMap<String, (f64, usize, usize)> = HashMap::new();
        for row in &self.rows {
            let raw = row.get(group_idx).map(|s| s.trim()).unwrap_or("");
            let key = if raw.is_empty() {
                "(빈값)".to_string()
            } else {
                raw.to_string()
            };
            let e = map.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                (0.0, 0, 0)
            });
            e.2 += 1; // 행 수
            if let Some(v) = row.get(value_idx).and_then(|s| parse_num(s)) {
                e.0 += v; // 합계
                e.1 += 1; // 숫자 개수
            }
        }
        let mut out: Vec<GroupStat> = order
            .into_iter()
            .map(|k| {
                let (sum, nc, rc) = map[&k];
                GroupStat {
                    key: k,
                    sum,
                    numeric_count: nc,
                    row_count: rc,
                }
            })
            .collect();
        out.sort_by(|a, b| {
            b.sum
                .partial_cmp(&a.sum)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }
}

/// 그룹별 집계 한 줄(group_by 결과).
pub struct GroupStat {
    pub key: String,
    pub sum: f64,
    pub numeric_count: usize,
    pub row_count: usize,
}

/// 여러 표를 머리글 기준으로 합친다(열 순서가 달라도 이름으로 맞춤).
/// 합친 머리글 = 첫 표 머리글 + 뒤 표에만 있는 새 열. 없는 칸은 빈값.
/// `with_source`면 맨 앞에 '출처'(각 표의 라벨) 열을 붙인다. (월별·지점별 파일 합본)
pub fn merge_tables(
    named: &[(String, &Table)],
    with_source: bool,
) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers: Vec<String> = Vec::new();
    for (_, t) in named {
        for h in &t.headers {
            let ht = h.trim();
            if !headers.iter().any(|x| x.trim().eq_ignore_ascii_case(ht)) {
                headers.push(h.clone());
            }
        }
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for (src, t) in named {
        let map: Vec<Option<usize>> = headers.iter().map(|h| t.col_index(h)).collect();
        for row in &t.rows {
            let mut out: Vec<String> = map
                .iter()
                .map(|m| m.and_then(|i| row.get(i)).cloned().unwrap_or_default())
                .collect();
            if with_source {
                out.insert(0, src.clone());
            }
            rows.push(out);
        }
    }
    if with_source {
        headers.insert(0, "출처".to_string());
    }
    (headers, rows)
}

/// 한 셀을 CSV 필드로(콤마·따옴표·줄바꿈 있으면 따옴표로 감싸고 내부 따옴표는 두 번).
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// 머리글 + 행들을 UTF-8 CSV 텍스트로 직렬화한다(엑셀 분석 결과 내보내기).
pub fn to_csv(headers: &[String], rows: &[Vec<String>]) -> String {
    let line = |cells: &[String]| {
        cells
            .iter()
            .map(|c| csv_field(c))
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut out = String::new();
    out.push_str(&line(headers));
    out.push('\n');
    for row in rows {
        out.push_str(&line(row));
        out.push('\n');
    }
    out
}

/// 필터 비교 연산자.
#[derive(Clone, Copy, PartialEq)]
enum FilterOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Contains,
}

impl Table {
    /// `열<연산>값` 식으로 행을 걸러 새 표를 만든다(원본 불변). 모든 분석 모드가
    /// 걸러진 부분집합 위에서 그대로 동작한다(필터 → 집계/정렬/통계 조합).
    /// 연산: `=` `!=` `>` `<` `>=` `<=` `~`(포함). 예: 지역=서울, 매출>1000000, 이름~상사.
    pub fn filtered(&self, expr: &str) -> Result<Table> {
        // 긴 연산자를 먼저 탐지(>=가 >·=보다, !=가 =보다 우선).
        let ops: [(&str, FilterOp); 7] = [
            (">=", FilterOp::Ge),
            ("<=", FilterOp::Le),
            ("!=", FilterOp::Ne),
            ("~", FilterOp::Contains),
            ("=", FilterOp::Eq),
            (">", FilterOp::Gt),
            ("<", FilterOp::Lt),
        ];
        let (col, op, val) = ops
            .iter()
            .find_map(|(s, o)| {
                expr.find(s)
                    .map(|pos| (expr[..pos].trim(), *o, expr[pos + s.len()..].trim()))
            })
            .ok_or_else(|| {
                anyhow!("필터 형식이 이상해요. 예: 지역=서울 · 매출>1000000 · 이름~상사")
            })?;
        if col.is_empty() {
            return Err(anyhow!("필터에 열 이름이 없어요. 예: 지역=서울"));
        }
        let idx = self.col_index(col).ok_or_else(|| {
            anyhow!(
                "'{col}' 열을 찾을 수 없어요. 열: {}",
                self.headers.join(", ")
            )
        })?;
        let val_num = parse_num(val);
        let matches = |cell: &str| -> bool {
            let cell = cell.trim();
            match op {
                FilterOp::Contains => cell.contains(val),
                FilterOp::Eq | FilterOp::Ne => {
                    let eq = cell.eq_ignore_ascii_case(val)
                        || parse_num(cell).zip(val_num).is_some_and(|(a, b)| a == b);
                    if op == FilterOp::Eq {
                        eq
                    } else {
                        !eq
                    }
                }
                _ => match (parse_num(cell), val_num) {
                    (Some(a), Some(b)) => match op {
                        FilterOp::Gt => a > b,
                        FilterOp::Lt => a < b,
                        FilterOp::Ge => a >= b,
                        FilterOp::Le => a <= b,
                        _ => false,
                    },
                    _ => false, // 숫자 비교인데 셀이 숫자가 아니면 제외
                },
            }
        };
        let rows: Vec<Vec<String>> = self
            .rows
            .iter()
            .filter(|row| matches(row.get(idx).map(|s| s.as_str()).unwrap_or("")))
            .cloned()
            .collect();
        Ok(Table {
            headers: self.headers.clone(),
            rows,
        })
    }

    /// `idx` 열 기준으로 정렬한 행 인덱스를 돌려준다(원본은 안 건드림).
    /// 숫자로 읽히는 값이 항상 먼저(값 순), 숫자 아닌(빈칸·텍스트) 행은 뒤로.
    /// `ascending`=false면 숫자는 큰 값부터(상위 N 보기의 기본).
    pub fn sorted_rows(&self, idx: usize, ascending: bool) -> Vec<usize> {
        use std::cmp::Ordering;
        let mut order: Vec<usize> = (0..self.rows.len()).collect();
        order.sort_by(|&a, &b| {
            let va = self.rows[a].get(idx).and_then(|s| parse_num(s));
            let vb = self.rows[b].get(idx).and_then(|s| parse_num(s));
            match (va, vb) {
                (Some(x), Some(y)) => {
                    let c = x.partial_cmp(&y).unwrap_or(Ordering::Equal);
                    if ascending {
                        c
                    } else {
                        c.reverse()
                    }
                }
                (Some(_), None) => Ordering::Less, // 숫자가 항상 먼저
                (None, Some(_)) => Ordering::Greater,
                (None, None) => {
                    let sa = self.rows[a].get(idx).map(|s| s.trim()).unwrap_or("");
                    let sb = self.rows[b].get(idx).map(|s| s.trim()).unwrap_or("");
                    if ascending {
                        sa.cmp(sb)
                    } else {
                        sb.cmp(sa)
                    }
                }
            }
        });
        order
    }
}

/// 통화기호·콤마·% 등을 떼고 숫자로 파싱.
fn parse_num(s: &str) -> Option<f64> {
    let s = s.trim();
    // 회계식 음수 표기 (1,000) → -1,000.
    let (s, neg) = if s.starts_with('(') && s.ends_with(')') && s.len() >= 2 {
        (&s[1..s.len() - 1], true)
    } else {
        (s, false)
    };
    // 통화·단위·구분 기호 제거(원: 한국 화폐 단위, ₩·$·%·콤마·공백).
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(c, ',' | '₩' | '$' | '%' | ' ' | '\t' | '원'))
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned
        .parse::<f64>()
        .ok()
        .map(|v| if neg { -v } else { v })
        // f64::parse는 "inf"/"infinity"/"nan"을 받아들여 합계·평균을 오염시킨다 → 유한값만 숫자로.
        .filter(|v| v.is_finite())
}

/// CSV(또는 TSV) 파일을 읽는다. UTF-8이 아니면 CP949(한국 엑셀·은행 CSV)로 폴백.
fn load_csv(path: &str) -> Result<Table> {
    let bytes = std::fs::read(path)?;
    let text = match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        // 윈도우 엑셀·은행 CSV는 CP949(EUC-KR)로 저장되는 경우가 많다.
        Err(_) => encoding_rs::EUC_KR.decode(&bytes).0.into_owned(),
    };
    // 엑셀이 UTF-8로 저장할 때 붙이는 BOM(U+FEFF)을 제거(첫 열 이름 매칭 실패 방지).
    let text = text
        .strip_prefix('\u{feff}')
        .map(str::to_string)
        .unwrap_or(text);
    let delim = if path.to_lowercase().ends_with(".tsv") {
        '\t'
    } else {
        ','
    };
    let mut records: Vec<Vec<String>> = Vec::new();
    for line in split_records(&text) {
        if line.is_empty() {
            continue;
        }
        records.push(parse_csv_line(&line, delim));
    }
    if records.is_empty() {
        return Err(anyhow!("빈 파일이에요: {path}"));
    }
    let headers = records.remove(0);
    Ok(Table {
        headers,
        rows: records,
    })
}

/// 따옴표 안의 줄바꿈을 고려하며 레코드 단위로 자른다.
fn split_records(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for c in text.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                cur.push(c);
            }
            '\n' if !in_quotes => {
                out.push(cur.trim_end_matches('\r').to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur.trim_end_matches('\r').to_string());
    }
    out
}

/// CSV 한 줄을 필드로 나눈다(따옴표 이스케이프 처리).
fn parse_csv_line(line: &str, delim: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            c if c == delim && !in_quotes => {
                fields.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    fields.push(cur.trim().to_string());
    fields
}

/// 엑셀 계열 파일을 calamine으로 읽는다(첫 시트).
/// 엑셀 파일의 시트 이름 목록(CSV·열기 실패 시 빈 Vec). 다중 시트 안내·선택에 쓴다.
pub fn list_sheets(path: &str) -> Vec<String> {
    use calamine::{open_workbook_auto, Reader};
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "xlsx" | "xls" | "xlsm" | "xlsb" | "ods") {
        return Vec::new();
    }
    match open_workbook_auto(path) {
        Ok(wb) => wb.sheet_names().to_vec(),
        Err(_) => Vec::new(),
    }
}

fn load_spreadsheet(path: &str, sheet: Option<&str>) -> Result<Table> {
    use calamine::{open_workbook_auto, Data, Reader};
    let mut wb = open_workbook_auto(path).map_err(|e| anyhow!("엑셀을 열 수 없어요: {e}"))?;
    let names: Vec<String> = wb.sheet_names().to_vec();
    if names.is_empty() {
        return Err(anyhow!("시트가 없어요"));
    }
    let range = match sheet.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        None => wb
            .worksheet_range_at(0)
            .ok_or_else(|| anyhow!("시트가 없어요"))?
            .map_err(|e| anyhow!("시트를 읽을 수 없어요: {e}"))?,
        Some(s) => {
            // 1-기반 번호 우선, 아니면 이름(대소문자·공백 무시) 매칭.
            match s
                .parse::<usize>()
                .ok()
                .filter(|n| *n >= 1 && *n <= names.len())
            {
                Some(n) => wb
                    .worksheet_range_at(n - 1)
                    .ok_or_else(|| anyhow!("시트가 없어요"))?
                    .map_err(|e| anyhow!("시트를 읽을 수 없어요: {e}"))?,
                None => match names.iter().find(|nm| nm.trim().eq_ignore_ascii_case(s)) {
                    Some(nm) => wb
                        .worksheet_range(nm)
                        .map_err(|e| anyhow!("시트를 읽을 수 없어요: {e}"))?,
                    None => {
                        return Err(anyhow!(
                            "'{s}' 시트를 찾을 수 없어요. 시트: {}",
                            names.join(" · ")
                        ))
                    }
                },
            }
        }
    };

    let cell = |d: &Data| -> String {
        match d {
            Data::Empty => String::new(),
            Data::String(s) => s.trim().to_string(),
            Data::Float(f) => {
                // i64 범위 밖 정수를 `as i64`로 캐스트하면 i64::MAX로 포화돼 조용히 손상되므로,
                // 안전 범위에서만 정수로 포맷하고 그 밖(경 단위 이상·19~20자리 ID 등)은 그대로 둔다.
                if f.fract() == 0.0 && f.abs() < 9.0e18 {
                    format!("{}", *f as i64)
                } else {
                    f.to_string()
                }
            }
            Data::Int(i) => i.to_string(),
            Data::Bool(b) => b.to_string(),
            other => other.to_string(),
        }
    };

    let mut rows = range.rows();
    let headers: Vec<String> = match rows.next() {
        Some(r) => r.iter().map(cell).collect(),
        None => return Err(anyhow!("빈 시트예요")),
    };
    let body: Vec<Vec<String>> = rows.map(|r| r.iter().map(cell).collect()).collect();
    Ok(Table {
        headers,
        rows: body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Table {
        Table {
            headers: vec!["이름".into(), "금액".into(), "수량".into()],
            rows: vec![
                vec!["철수".into(), "1,000".into(), "3".into()],
                vec!["영희".into(), "2500".into(), "5".into()],
                vec!["민수".into(), "₩3,000".into(), "".into()],
            ],
        }
    }

    #[test]
    fn finds_column_by_name_or_number() {
        let t = sample();
        assert_eq!(t.col_index("금액"), Some(1));
        assert_eq!(t.col_index("2"), Some(1));
        assert_eq!(t.col_index("없는열"), None);
    }

    #[test]
    fn parse_num_handles_korean_money() {
        // 천단위 콤마 + 통화기호.
        assert_eq!(parse_num("1,000"), Some(1000.0));
        assert_eq!(parse_num("₩3,000"), Some(3000.0));
        // 원 접미사(한국 가계부·은행 CSV에 흔함).
        assert_eq!(parse_num("5,000원"), Some(5000.0));
        assert_eq!(parse_num("12,000 원"), Some(12000.0));
        // 회계식 괄호 음수.
        assert_eq!(parse_num("(1,000)"), Some(-1000.0));
        // 음수·소수.
        assert_eq!(parse_num("-2,500.5"), Some(-2500.5));
        // 진짜 텍스트는 여전히 None.
        assert_eq!(parse_num("원룸"), None);
        assert_eq!(parse_num(""), None);
        // inf/nan(대소문자 무관)을 숫자로 받으면 합계·평균이 오염되므로 None이어야.
        assert_eq!(parse_num("inf"), None);
        assert_eq!(parse_num("INFINITY"), None);
        assert_eq!(parse_num("-inf"), None);
        assert_eq!(parse_num("nan"), None);
        assert_eq!(parse_num("NaN"), None);
    }

    #[test]
    fn numeric_column_strips_symbols() {
        let t = sample();
        let nums = t.numeric_column(1);
        assert_eq!(nums, vec![1000.0, 2500.0, 3000.0]);
        // 빈 칸은 건너뛴다.
        assert_eq!(t.numeric_column(2), vec![3.0, 5.0]);
    }

    #[test]
    fn group_by_sums_and_sorts() {
        // 지점별 매출: 강남 1,000+2,000원, 홍대 500(+'미정' 제외), 빈 지점 1건
        let t = Table {
            headers: vec!["지점".into(), "매출".into()],
            rows: vec![
                vec!["강남".into(), "1,000원".into()],
                vec!["홍대".into(), "500".into()],
                vec!["강남".into(), "2,000원".into()],
                vec!["홍대".into(), "미정".into()],
                vec!["".into(), "300".into()],
            ],
        };
        let g = t.group_by(0, 1);
        // 합계 내림차순: 강남(3000) > 홍대(500) > (빈값)(300)
        assert_eq!(g[0].key, "강남");
        assert_eq!(g[0].sum, 3000.0);
        assert_eq!(g[0].numeric_count, 2);
        assert_eq!(g[0].row_count, 2);
        assert_eq!(g[1].key, "홍대");
        assert_eq!(g[1].sum, 500.0);
        assert_eq!(g[1].numeric_count, 1); // '미정'은 숫자 아님 → 제외
        assert_eq!(g[1].row_count, 2);
        assert_eq!(g[2].key, "(빈값)"); // 빈 그룹값
        assert_eq!(g[2].sum, 300.0);
    }

    #[test]
    fn merge_tables_aligns_headers_and_source() {
        let jan = Table {
            headers: vec!["지점".into(), "매출".into()],
            rows: vec![vec!["강남".into(), "100".into()]],
        };
        // 2월은 열 순서가 다르고 '비고'가 추가됨.
        let feb = Table {
            headers: vec!["매출".into(), "지점".into(), "비고".into()],
            rows: vec![vec!["200".into(), "홍대".into(), "신규".into()]],
        };
        let (headers, rows) = merge_tables(&[("1월".into(), &jan), ("2월".into(), &feb)], true);
        // 출처 + 합집합 헤더(첫 표 순서 유지, 새 열 뒤에).
        assert_eq!(headers, vec!["출처", "지점", "매출", "비고"]);
        // 1월 행: 비고 없음 → 빈값, 이름으로 정렬됨.
        assert_eq!(rows[0], vec!["1월", "강남", "100", ""]);
        // 2월 행: 열 순서 달라도 이름으로 맞춰짐.
        assert_eq!(rows[1], vec!["2월", "홍대", "200", "신규"]);
    }

    #[test]
    fn to_csv_quotes_and_roundtrips() {
        let headers = vec!["이름".to_string(), "메모".to_string()];
        let rows = vec![
            vec!["철수".to_string(), "서울, 강남".to_string()], // 콤마 → 따옴표
            vec!["영희".to_string(), "말하길 \"안녕\"".to_string()], // 따옴표 → 두 번
        ];
        let csv = to_csv(&headers, &rows);
        assert!(csv.contains("\"서울, 강남\""));
        assert!(csv.contains("\"말하길 \"\"안녕\"\"\""));
        // 다시 파싱하면 원래 값 복구(라운드트립).
        let recs = split_records(&csv);
        assert_eq!(parse_csv_line(&recs[0], ','), vec!["이름", "메모"]);
        assert_eq!(parse_csv_line(&recs[1], ','), vec!["철수", "서울, 강남"]);
        assert_eq!(
            parse_csv_line(&recs[2], ','),
            vec!["영희", "말하길 \"안녕\""]
        );
    }

    #[test]
    fn filtered_supports_ops() {
        let t = Table {
            headers: vec!["지역".into(), "매출".into()],
            rows: vec![
                vec!["서울".into(), "1,000".into()],
                vec!["부산".into(), "3,000".into()],
                vec!["서울".into(), "2,000".into()],
                vec!["대구".into(), "500".into()],
            ],
        };
        // 문자열 일치
        assert_eq!(t.filtered("지역=서울").unwrap().rows.len(), 2);
        assert_eq!(t.filtered("지역!=서울").unwrap().rows.len(), 2);
        // 포함
        assert_eq!(t.filtered("지역~부").unwrap().rows.len(), 1);
        // 숫자 비교(콤마 든 값도 파싱)
        assert_eq!(t.filtered("매출>1000").unwrap().rows.len(), 2); // 3000,2000
        assert_eq!(t.filtered("매출>=2,000").unwrap().rows.len(), 2);
        assert_eq!(t.filtered("매출<1000").unwrap().rows.len(), 1); // 500
                                                                    // 없는 열 / 형식 오류
        assert!(t.filtered("부서=영업").is_err());
        assert!(t.filtered("지역서울").is_err());
    }

    #[test]
    fn sorted_rows_numeric_desc_and_text_last() {
        let t = Table {
            headers: vec!["지점".into(), "매출".into()],
            rows: vec![
                vec!["A".into(), "1,000".into()],
                vec!["B".into(), "3,000".into()],
                vec!["C".into(), "미정".into()], // 숫자 아님 → 뒤로
                vec!["D".into(), "2,000".into()],
            ],
        };
        // 기본(큰 값부터): B(3000) > D(2000) > A(1000) > C(텍스트)
        assert_eq!(t.sorted_rows(1, false), vec![1, 3, 0, 2]);
        // 오름차순: A(1000) < D(2000) < B(3000), 텍스트는 여전히 끝
        assert_eq!(t.sorted_rows(1, true), vec![0, 3, 1, 2]);
        // 텍스트 열(지점) 오름차순 = 가나다(A<B<C<D)
        assert_eq!(t.sorted_rows(0, true), vec![0, 1, 2, 3]);
    }

    #[test]
    fn parses_quoted_csv_line() {
        let f = parse_csv_line(r#"철수,"서울, 강남","말하길 ""안녕""""#, ',');
        assert_eq!(f, vec!["철수", "서울, 강남", r#"말하길 "안녕""#]);
    }

    #[test]
    fn splits_records_respecting_quotes() {
        let recs = split_records("a,b\n\"여러\n줄\",c\n");
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[1], "\"여러\n줄\",c");
    }
}

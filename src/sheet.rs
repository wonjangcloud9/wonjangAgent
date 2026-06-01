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
    /// 확장자에 따라 CSV 또는 엑셀로 읽는다.
    pub fn load(path: &str) -> Result<Table> {
        if !Path::new(path).exists() {
            return Err(anyhow!("파일을 찾을 수 없어요: {path}"));
        }
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "xlsx" | "xls" | "xlsm" | "xlsb" | "ods" => load_spreadsheet(path),
            _ => load_csv(path),
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
}

/// 통화기호·콤마·% 등을 떼고 숫자로 파싱.
fn parse_num(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(c, ',' | '₩' | '$' | '%' | ' ' | '\t'))
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

/// CSV(또는 TSV) 파일을 읽는다.
fn load_csv(path: &str) -> Result<Table> {
    let text = std::fs::read_to_string(path)?;
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
fn load_spreadsheet(path: &str) -> Result<Table> {
    use calamine::{open_workbook_auto, Data, Reader};
    let mut wb = open_workbook_auto(path).map_err(|e| anyhow!("엑셀을 열 수 없어요: {e}"))?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow!("시트가 없어요"))?
        .map_err(|e| anyhow!("시트를 읽을 수 없어요: {e}"))?;

    let cell = |d: &Data| -> String {
        match d {
            Data::Empty => String::new(),
            Data::String(s) => s.trim().to_string(),
            Data::Float(f) => {
                if f.fract() == 0.0 {
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
    fn numeric_column_strips_symbols() {
        let t = sample();
        let nums = t.numeric_column(1);
        assert_eq!(nums, vec![1000.0, 2500.0, 3000.0]);
        // 빈 칸은 건너뛴다.
        assert_eq!(t.numeric_column(2), vec![3.0, 5.0]);
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

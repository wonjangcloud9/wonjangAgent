//! 사칙연산 계산기 — 터미널에서 식을 바로 계산한다.
//!
//! `+ - * / ( )`, 소수, 음수, 공백을 지원하는 재귀 하향 파서. 외부 의존성·키가
//! 없다. 우선순위(곱·나눗셈 먼저)와 괄호를 정확히 처리한다.

use anyhow::{anyhow, Result};

/// 식 문자열을 계산한다.
pub fn eval(expr: &str) -> Result<f64> {
    let tokens = tokenize(expr)?;
    let mut parser = Parser { tokens, pos: 0 };
    let value = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() {
        return Err(anyhow!("식을 해석할 수 없어요(연산자/괄호를 확인하세요)"));
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => i += 1, // 공백 무시
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' | 'x' | 'X' | '×' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' | '÷' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            d if d.is_ascii_digit() || d == '.' => {
                let start = i;
                // 천 단위 콤마는 숫자 일부로 보고 함께 읽는다.
                while i < chars.len()
                    && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',')
                {
                    i += 1;
                }
                let num: String = chars[start..i].iter().filter(|c| **c != ',').collect();
                let val = num
                    .parse::<f64>()
                    .map_err(|_| anyhow!("숫자를 해석할 수 없어요: {num}"))?;
                tokens.push(Token::Num(val));
            }
            other => return Err(anyhow!("계산할 수 없는 문자: '{other}'")),
        }
    }
    if tokens.is_empty() {
        return Err(anyhow!("계산할 식을 입력하세요. 예: 15000 * 1.1"));
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    // expr := term (('+'|'-') term)*
    fn parse_expr(&mut self) -> Result<f64> {
        let mut value = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.pos += 1;
                    value += self.parse_term()?;
                }
                Token::Minus => {
                    self.pos += 1;
                    value -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    // term := factor (('*'|'/') factor)*
    fn parse_term(&mut self) -> Result<f64> {
        let mut value = self.parse_factor()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.pos += 1;
                    value *= self.parse_factor()?;
                }
                Token::Slash => {
                    self.pos += 1;
                    let divisor = self.parse_factor()?;
                    if divisor == 0.0 {
                        return Err(anyhow!("0으로 나눌 수 없어요"));
                    }
                    value /= divisor;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    // factor := number | '(' expr ')' | '-' factor | '+' factor
    fn parse_factor(&mut self) -> Result<f64> {
        match self.peek().cloned() {
            Some(Token::Num(n)) => {
                self.pos += 1;
                Ok(n)
            }
            Some(Token::Minus) => {
                self.pos += 1;
                Ok(-self.parse_factor()?)
            }
            Some(Token::Plus) => {
                self.pos += 1;
                self.parse_factor()
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let value = self.parse_expr()?;
                match self.peek() {
                    Some(Token::RParen) => {
                        self.pos += 1;
                        Ok(value)
                    }
                    _ => Err(anyhow!("닫는 괄호 ')'가 없어요")),
                }
            }
            _ => Err(anyhow!("식이 올바르지 않아요")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> f64 {
        eval(s).unwrap()
    }

    #[test]
    fn precedence_and_parens() {
        assert_eq!(ok("1 + 2 * 3"), 7.0);
        assert_eq!(ok("(1 + 2) * 3"), 9.0);
        assert_eq!(ok("2 * (3 + 4) - 5"), 9.0);
    }

    #[test]
    fn decimals_and_negatives() {
        assert!((ok("15000 * 1.1") - 16_500.0).abs() < 1e-9);
        assert_eq!(ok("-3 + 5"), 2.0);
        assert_eq!(ok("10 / 4"), 2.5);
    }

    #[test]
    fn alt_symbols_and_commas() {
        assert_eq!(ok("3 x 4"), 12.0);
        assert_eq!(ok("1,000 + 2,000"), 3_000.0);
    }

    #[test]
    fn errors() {
        assert!(eval("1 / 0").is_err());
        assert!(eval("(1 + 2").is_err());
        assert!(eval("1 +").is_err());
        assert!(eval("abc").is_err());
    }
}

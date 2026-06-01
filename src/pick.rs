//! 제비뽑기 / 랜덤 추첨 — "누가 걸릴까", 순서 정하기 등 모임 결정 도우미.
//!
//! 시간 기반 시드로 Fisher-Yates 셔플을 돌린다. 외부 의존성·키가 없다.

use std::time::{SystemTime, UNIX_EPOCH};

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Fisher-Yates 셔플로 항목 순서를 섞는다(시드 결정적).
pub fn shuffle<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
    let mut out = items.to_vec();
    let mut state = seed | 1;
    let n = out.len();
    if n < 2 {
        return out;
    }
    // i를 끝에서부터 내려오며 0..=i 중 하나와 교환.
    for i in (1..n).rev() {
        let j = (xorshift(&mut state) % (i as u64 + 1)) as usize;
        out.swap(i, j);
    }
    out
}

/// 항목 중 `count`개를 무작위로 뽑는다(중복 없음). 시간 기반 시드.
pub fn draw<T: Clone>(items: &[T], count: usize) -> Vec<T> {
    let shuffled = shuffle(items, now_nanos());
    shuffled.into_iter().take(count.min(items.len())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_preserves_elements() {
        let items = vec!["철수", "영희", "민수", "지은"];
        let mut s = shuffle(&items, 12345);
        s.sort();
        let mut original = items.clone();
        original.sort();
        assert_eq!(s, original);
    }

    #[test]
    fn shuffle_is_deterministic() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(shuffle(&items, 42), shuffle(&items, 42));
    }

    #[test]
    fn single_item_unchanged() {
        assert_eq!(shuffle(&["혼자"], 7), vec!["혼자"]);
    }

    #[test]
    fn draw_count_is_clamped() {
        let items = vec!["a", "b", "c"];
        assert_eq!(draw(&items, 10).len(), 3);
        assert_eq!(draw(&items, 2).len(), 2);
    }
}

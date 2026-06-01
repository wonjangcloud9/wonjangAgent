//! 로또 자동 번호 추첨기 — 재미로 쓰는 1~45 중 6개 자동 번호 생성.
//!
//! 외부 의존성 없이 시간 기반 시드로 간단한 xorshift PRNG를 돌린다(암호용 아님).

use std::collections::BTreeSet;
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

/// 시드로부터 로또 한 게임(1~45 중 서로 다른 6개, 오름차순).
pub fn game(seed: u64) -> Vec<u32> {
    let mut state = seed | 1; // 0 시드 방지
    let mut set: BTreeSet<u32> = BTreeSet::new();
    while set.len() < 6 {
        let n = (xorshift(&mut state) % 45) as u32 + 1;
        set.insert(n);
    }
    set.into_iter().collect()
}

/// N게임 자동 생성(시간 기반 시드, 게임마다 다르게).
pub fn auto(games: usize) -> Vec<Vec<u32>> {
    let base = now_nanos();
    (0..games)
        .map(|i| game(base.wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_is_valid() {
        let g = game(123_456_789);
        assert_eq!(g.len(), 6);
        assert!(g.iter().all(|n| (1..=45).contains(n)));
        // 오름차순 + 중복 없음.
        assert!(g.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn game_is_deterministic() {
        assert_eq!(game(42), game(42));
    }

    #[test]
    fn auto_generates_n_games() {
        let games = auto(5);
        assert_eq!(games.len(), 5);
        assert!(games.iter().all(|g| g.len() == 6));
    }
}

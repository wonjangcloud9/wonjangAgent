//! 자동차세 계산 — 비영업용 승용차 기준(지방세법). 순수 공식, 키 불필요.
//!
//! 자동차세(본세) = 배기량 × cc단가(1,000cc 이하 80원 / 1,600cc 이하 140원 / 초과 200원).
//! 차령 3년차부터 매년 5%씩 경감(최대 50%). 지방교육세 = 본세의 30%.
//! cc단가·경감율은 오래 안정적이라 staleness가 거의 없다.

/// 비영업용 승용차 연 세액을 `(자동차세 본세, 지방교육세)` 원 단위로. 배기량 cc, 차령(년).
pub fn annual_tax(cc: u32, age_years: u32) -> (i64, i64) {
    let per_cc: i64 = if cc <= 1000 {
        80
    } else if cc <= 1600 {
        140
    } else {
        200
    };
    let base = cc as i64 * per_cc;
    // 차령 경감: 3년차부터 5%/년, 상한 50%(차령 12년). 곱하기 전에 10으로 캡 → 거대 입력 오버플로 방지.
    let discount = if age_years >= 3 {
        ((age_years - 2).min(10) * 5) as i64
    } else {
        0
    };
    let tax = base * (100 - discount) / 100;
    let edu = tax * 30 / 100;
    (tax, edu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values_new_car() {
        // 잘 알려진 실제 값과 대조(신차 기준).
        assert_eq!(annual_tax(2000, 0), (400_000, 120_000)); // 합 52만
        assert_eq!(annual_tax(1600, 0), (224_000, 67_200));
        assert_eq!(annual_tax(1000, 0), (80_000, 24_000));
        assert_eq!(annual_tax(998, 0), (79_840, 23_952)); // 1000cc 이하 80원
        assert_eq!(annual_tax(1598, 0).0, 1598 * 140); // 1600cc 이하 140원
    }

    #[test]
    fn age_discount() {
        assert_eq!(annual_tax(2000, 5).0, 340_000); // 15% 경감
        assert_eq!(annual_tax(2000, 12).0, 200_000); // 50% 상한
        assert_eq!(annual_tax(2000, 20).0, 200_000); // 상한 유지
        assert_eq!(annual_tax(2000, 2).0, 400_000); // 2년 이하 경감 없음
        assert_eq!(annual_tax(2000, 3).0, 380_000); // 3년차 5%
        assert_eq!(annual_tax(2000, u32::MAX).0, 200_000); // 거대 차령도 오버플로 없이 상한
    }
}

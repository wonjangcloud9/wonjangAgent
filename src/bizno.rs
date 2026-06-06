//! 사업자등록번호 검증 — 국세청 검증번호(체크섬) 알고리즘.
//!
//! 세금계산서·계약서·거래처 등록 시 사업자번호 오타·허위를 잡는다. 10자리의
//! 가중치 합으로 마지막 검증 숫자를 확인한다(공개 알고리즘, 키·네트워크 없음).
//! 주의: 검증번호가 맞아도 *실제 사업자 존재*는 국세청 조회(별도 키)가 필요하다 —
//! 여기선 형식·체크섬만 본다(오타 거르기엔 충분).

/// 사업자등록번호 유효성. 10자리 숫자가 아니면 None, 맞으면 Some(검증번호 일치 여부).
/// 하이픈·공백은 무시한다(`124-81-00998`·`1248100998` 모두 OK).
pub fn is_valid(input: &str) -> Option<bool> {
    let digits: Vec<u32> = input.chars().filter_map(|c| c.to_digit(10)).collect();
    // 숫자가 아닌(하이픈·공백 외) 문자가 섞이면 입력 오류로 본다.
    if digits.len() != 10
        || input
            .chars()
            .any(|c| !c.is_ascii_digit() && !matches!(c, '-' | ' '))
    {
        return None;
    }
    // 전부 0(000-00-00000)은 체크섬상 통과하지만 실재하지 않는 번호다 — 무효로 본다.
    if digits.iter().all(|&d| d == 0) {
        return Some(false);
    }
    const W: [u32; 9] = [1, 3, 7, 1, 3, 7, 1, 3, 5];
    let mut sum: u32 = (0..9).map(|i| digits[i] * W[i]).sum();
    sum += (digits[8] * 5) / 10; // 9번째 자리 ×5의 십의 자리를 더함
    let check = (10 - (sum % 10)) % 10;
    Some(check == digits[9])
}

/// `1248100998`을 `124-81-00998` 꼴로(10자리일 때). 아니면 원본.
pub fn format(input: &str) -> String {
    let d: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() == 10 {
        format!("{}-{}-{}", &d[0..3], &d[3..5], &d[5..10])
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_real_numbers() {
        // 공개 사업자번호(체크섬 검증).
        assert_eq!(is_valid("124-81-00998"), Some(true)); // 삼성전자
        assert_eq!(is_valid("1248100998"), Some(true)); // 하이픈 없이
        assert_eq!(is_valid("120-81-47521"), Some(true)); // 카카오
                                                          // 끝자리 오타 → 검증번호 불일치.
        assert_eq!(is_valid("124-81-00997"), Some(false));
        // 전부 0은 체크섬상 통과하지만 실재하지 않으므로 무효.
        assert_eq!(is_valid("000-00-00000"), Some(false));
        assert_eq!(is_valid("0000000000"), Some(false));
        // 자리수 부족·초과·문자 → None.
        assert_eq!(is_valid("124-81-0099"), None);
        assert_eq!(is_valid("12481009980"), None);
        assert_eq!(is_valid("124-81-0099a"), None);
    }

    #[test]
    fn formats_with_hyphens() {
        assert_eq!(format("1248100998"), "124-81-00998");
        assert_eq!(format("124-81-00998"), "124-81-00998");
    }
}

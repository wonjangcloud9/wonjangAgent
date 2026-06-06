# 🛠 도구

개발·일상에 쓰는 도구들. 로컬에서 바로 돌아가요(키·네트워크 불필요).

## 보안·생성

```bash
wonjang 비밀번호 16                 # OS 난수로 안전한 비밀번호
wonjang qr "https://example.com"   # QR 코드 생성
wonjang 해시 파일.zip               # SHA-256 체크섬
wonjang 인코딩 base64 "hello"       # base64·URL 인코딩/디코딩
```

## 파일·데이터

```bash
wonjang 엑셀 데이터.csv             # CSV 분석·피벗
wonjang 찾기 "TODO" .              # 파일 내용 검색(grep)
wonjang 정리 ~/Downloads           # 폴더 자동 분류(미리보기 → --실행)
wonjang 용량 ~                     # 큰 파일·폴더 찾기
```

## 백업·복원

```bash
wonjang 백업                       # 모든 데이터를 한 파일로
wonjang 복원 backup.json           # 복원(복원 전 자동 백업)
```

## 한눈에 보기

```bash
wonjang 현황                       # 약속·할일·디데이·습관·집중·지출 한 화면
wonjang 도움                       # 전체 기능 카테고리별 안내
```

::: tip 전체 목록
원장은 100가지가 넘는 기능이 있어요. 카테고리별 전체 목록은 언제든 `wonjang 도움`에서 볼 수 있습니다.
:::

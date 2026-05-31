# 기여 가이드

## 개발 환경

Rust 툴체인([rustup.rs](https://rustup.rs))이 필요합니다.

```bash
cargo build              # 디버그 빌드
cargo test               # 단위 테스트
cargo test -- --ignored  # 네트워크 라이브 테스트(web 2종, mcp 1종 — python3 필요)
cargo fmt                # 자동 포맷
cargo clippy --all-targets -- -D warnings
```

PR을 올리기 전에 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`가 모두 통과하는지 확인해 주세요. CI도 동일하게 검사합니다.

## 프로젝트 구조

`src/` 아래 모듈로 구성됩니다(자세한 트리는 README의 "구조" 참고). 새 도구는
`src/tools/`에 추가하고 `tools::default_tools()`(또는 `subagent_tools()`)에 등록합니다.

## 릴리스 절차

릴리스는 git 태그로 트리거됩니다.

1. **버전 올리기** — `Cargo.toml`의 `version`을 새 버전(예: `0.2.0`)으로 수정.
   (npm 패키지 버전은 릴리스 워크플로가 태그에서 자동으로 맞춥니다.)
2. 변경 사항을 커밋하고 main에 푸시.
3. **태그 푸시**:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
4. `Release` 워크플로가 자동으로:
   - macOS(arm64/x64) · Linux(x64 musl) · Windows(x64) 바이너리를 빌드
   - `wonjang-<target>` 자산을 GitHub Release에 업로드
   - `npm/` 패키지 버전을 태그에 맞춰 npm에 배포

### 필요한 시크릿

- `NPM_TOKEN` — npm 자동 배포용 토큰(레포 Settings → Secrets → Actions에 등록).
  자동 배포가 필요 없으면 `release.yml`의 `publish-npm` 잡을 제거해도 됩니다.

`GITHUB_TOKEN`은 Actions에서 자동 제공되므로 릴리스 업로드에 별도 설정이 필요 없습니다.

## 커밋 메시지

`feat:`, `fix:`, `docs:`, `refactor:` 등 접두사를 사용합니다(Conventional Commits 권장).

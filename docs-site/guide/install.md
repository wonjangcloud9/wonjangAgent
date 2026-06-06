# 설치

## npm (권장)

```bash
npm install -g wonjang-agent
```

설치하면 `wonjang` 명령이 생깁니다. 플랫폼에 맞는 네이티브 바이너리를 자동으로 내려받아요.

설치 확인:

```bash
wonjang --version
```

## 소스에서 빌드 (Rust)

지원되지 않는 환경이거나 최신 개발본을 쓰고 싶다면 소스에서 직접 빌드합니다.

```bash
# 저장소에서 바로 설치
cargo install --git https://github.com/wonjangcloud9/wonjangAgent
```

```bash
# 또는 클론 후 빌드
git clone https://github.com/wonjangcloud9/wonjangAgent
cd wonjangAgent
cargo install --path .
```

## 지원 플랫폼

| OS | 아키텍처 |
| --- | --- |
| macOS | arm64 (Apple Silicon) · x64 |
| Linux | x64 (musl 정적) |
| Windows | x64 |

::: info 순수 Rust · 단일 바이너리
원장은 순수 Rust로 만들어졌어요. 무거운 런타임 없이 **단일 바이너리**로 동작해 빠르고 가볍습니다.
Linux는 musl 정적 빌드라 의존성 걱정이 없어요.
:::

## 업데이트

```bash
npm update -g wonjang-agent
```

## 데이터는 어디에?

원장이 만드는 데이터(가계부·습관·디데이·성격 등)는 운영체제의 표준 위치에 로컬로 저장돼요.

- **macOS:** `~/Library/Application Support/wonjang/`
- **Linux:** `~/.local/share/wonjang/`

언제든 `wonjang 백업`으로 한 파일로 묶고, `wonjang 복원`으로 되돌릴 수 있어요.

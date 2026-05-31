# 원장 에이전트 (wonjangAgent)

> 로컬 환경을 다루는 **한국어 우선** 자율 AI 에이전트 — Rust로 작성.

[헤르메스 에이전트(NousResearch/hermes-agent)](https://github.com/NousResearch/hermes-agent)의
핵심 아이디어를 러스트로 재구성합니다: **제공자(provider) 무관 LLM**, **로컬 도구**,
**에이전트 루프**, 그리고 한국 사용자를 위한 **한국어 우선 UX**.

단일 바이너리로 빠르게 실행되며, 별도 런타임 없이 여러분의 컴퓨터에서 파일과
셸을 직접 다뤄 작업을 수행합니다.

---

## 특징

- 🦀 **단일 러스트 바이너리** — 의존 런타임 없이 빠르게 시작.
- 🇰🇷 **한국어 우선** — 모든 메시지·프롬프트·도움말이 한국어.
- 🔌 **제공자 무관** — OpenAI 호환 엔드포인트면 무엇이든(OpenRouter, OpenAI,
  DeepSeek, 로컬 vLLM 등). 모델은 설정 한 줄로 교체.
- 🛠 **로컬 도구** — 셸 실행, 파일 읽기/쓰기, 디렉터리 목록.
- 🧠 **영속 메모리** — 사용자/환경에 대해 배운 사실을 디스크에 저장하고 다음
  세션에 자동으로 불러와 "함께 성장"합니다.
- 💾 **세션 저장/이어가기** — 대화가 자동 저장되어 `--continue`로 언제든 이어집니다.
- 🛡 **안전 우선** — 셸 명령은 기본적으로 실행 전 사용자 승인을 요청(`--yes`로 생략).
- 💬 **대화형 REPL + 단발 실행** 모두 지원.

## 설치

Rust 툴체인이 필요합니다([rustup.rs](https://rustup.rs)).

```bash
git clone https://github.com/wonjangcloud9/wonjangAgent.git
cd wonjangAgent
cargo build --release
# 결과물: ./target/release/wonjang
```

선택: PATH에 추가

```bash
cargo install --path .
```

## 설정

API 키는 보안을 위해 **환경 변수**로만 받습니다(설정 파일에 저장하지 않음).

```bash
# OpenRouter 예시
export OPENROUTER_API_KEY=sk-or-...
export WONJANG_MODEL=anthropic/claude-3.5-sonnet

# 또는 OpenAI 직접
export WONJANG_BASE_URL=https://api.openai.com/v1
export OPENAI_API_KEY=sk-...
export WONJANG_MODEL=gpt-4o
```

| 환경 변수 | 설명 | 기본값 |
| --- | --- | --- |
| `WONJANG_BASE_URL` | OpenAI 호환 베이스 URL | `https://openrouter.ai/api/v1` |
| `WONJANG_MODEL` | 모델 이름 | `anthropic/claude-3.5-sonnet` |
| `WONJANG_API_KEY` | API 키(없으면 `OPENROUTER_API_KEY` → `OPENAI_API_KEY` 폴백) | — |

현재 설정 확인 / 기본 설정 파일 생성:

```bash
wonjang config
```

## 사용법

**대화형 모드**

```bash
wonjang
```

```
  원장 에이전트  v0.1.0
  로컬 환경을 다루는 한국어 우선 AI 에이전트
  모델: anthropic/claude-3.5-sonnet
  도움말은 /help, 종료는 /exit 또는 Ctrl-D

당신 ▸ 이 폴더에 뭐가 있는지 알려줘
```

**단발 실행**

```bash
wonjang "git 상태 알려줘"
wonjang -y "현재 폴더 파일을 종류별로 정리하는 스크립트 짜줘"   # -y: 자동 승인
wonjang --continue "아까 하던 거 계속하자"                      # -c: 직전 대화 이어가기
```

**세션 관리**

```bash
wonjang sessions      # 저장된 대화 목록(최신순)
wonjang --continue    # 가장 최근 대화 이어가기
```

**REPL 슬래시 명령**

| 명령 | 설명 |
| --- | --- |
| `/help` | 도움말 |
| `/reset` | 대화 기록 초기화 |
| `/exit` | 종료 |

> 💡 **팁:** 구체적으로 요청할수록 잘 동작합니다. "코드 고쳐줘"보다
> "api/handlers.py 47번째 줄 TypeError 고쳐줘"처럼 명확히 말해 주세요.

## 도구

| 도구 | 설명 |
| --- | --- |
| `run_shell` | 셸 명령 실행(기본: 실행 전 승인) |
| `read_file` | 파일 내용 읽기 |
| `write_file` | 파일 쓰기(상위 폴더 자동 생성) |
| `list_dir` | 디렉터리 목록 |
| `remember` | 기억할 사실을 영속 메모리에 저장 |
| `recall` | 저장된 메모리 회상 |

저장된 메모리는 언제든 확인할 수 있습니다:

```bash
wonjang memory
```

## 구조

```
src/
├── main.rs       # CLI 진입점 + 대화형 REPL
├── agent.rs      # 에이전트 루프(LLM ↔ 도구 왕복)
├── llm.rs        # OpenAI 호환 LLM 클라이언트
├── config.rs     # 설정 로딩(환경 변수 > 파일 > 기본값)
├── memory.rs     # 영속 메모리(세션 간 사실 유지)
├── session.rs    # 세션 저장/이어가기
├── ui.rs         # 한국어 우선 터미널 UI
└── tools/        # 로컬 환경 도구
    ├── shell.rs
    ├── fs.rs
    └── memory.rs
```

## 로드맵

헤르메스 에이전트에 준하는 기능을 점진적으로 추가합니다:

- [x] v0.1 — 에이전트 루프, 제공자 무관 LLM, 셸/파일 도구, 한국어 REPL
- [x] v0.2 — 영속 메모리(remember/recall, 세션 간 사실 유지)
- [x] v0.3 — 세션 저장/이어가기(`--continue`, `wonjang sessions`)
- [ ] 자동 스킬 생성/재사용
- [ ] 웹/브라우저 도구, 웹 검색
- [ ] 크론 스케줄러(무인 자동화)
- [ ] 서브에이전트, MCP 연동
- [ ] 메시징 게이트웨이(텔레그램/슬랙 등)

## 라이선스

MIT

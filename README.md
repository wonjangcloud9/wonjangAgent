# 원장 에이전트 (wonjangAgent)

> 로컬 환경을 다루는 **한국어 우선** 자율 AI 에이전트 — Rust로 작성.

**제공자(provider) 무관 LLM**, **로컬 도구**, **에이전트 루프**, 그리고 한국
사용자를 위한 **한국어 우선 UX**를 갖춘 자율 에이전트입니다.

단일 바이너리로 빠르게 실행되며, 별도 런타임 없이 여러분의 컴퓨터에서 파일과
셸을 직접 다뤄 작업을 수행합니다.

---

## 특징

- 🦀 **단일 러스트 바이너리** — 의존 런타임 없이 빠르게 시작.
- 🇰🇷 **한국어 우선** — 모든 메시지·프롬프트·도움말이 한국어.
- 🔌 **제공자 무관** — OpenAI 호환 엔드포인트면 무엇이든(OpenRouter, OpenAI,
  DeepSeek, 로컬 vLLM 등). 모델은 설정 한 줄로 교체.
- 🛠 **로컬 도구** — 셸 실행, 파일 읽기/쓰기, 디렉터리 목록.
- 🌐 **웹 검색/가져오기** — 별도 API 키 없이 웹을 검색하고 페이지 본문을 읽어옵니다.
- 👥 **서브에이전트** — 큰 작업을 격리된 하위 에이전트에 위임하고, 여러 개를
  병렬로 동시에 처리합니다.
- 🔗 **MCP 연동** — 외부 MCP 서버(파일시스템·깃허브 등)를 연결해 그 도구들을
  원장 도구로 바로 사용합니다.
- 📱 **텔레그램 게이트웨이** — 휴대폰 등 어디서든 메시지로 원장에게 작업을
  시키고 결과를 받습니다(허용 chat_id 화이트리스트로 보호).
- 🧠 **영속 메모리** — 사용자/환경에 대해 배운 사실을 디스크에 저장하고 다음
  세션에 자동으로 불러와 "함께 성장"합니다.
- 📚 **스킬(절차 기억)** — 까다로운 작업을 해결하면 그 방법을 스킬로 저장하고,
  비슷한 일을 만나면 스스로 꺼내 재사용합니다.
- 💾 **세션 저장/이어가기** — 대화가 자동 저장되어 `--continue`로 언제든 이어집니다.
- ⏰ **예약 작업(크론)** — "매일", "30분마다" 같은 일정으로 작업을 무인 실행합니다.
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

**예약 작업(크론)**

일정에 맞춰 작업을 무인 실행합니다. 스케줄은 간격 기반입니다 —
`@minutely` · `@hourly` · `@daily` · `@weekly`, 또는 `@every 30m`, `2h`, `1d` 처럼 지정.

```bash
wonjang cron add "@daily" "오늘 한 git 커밋들 요약해줘"
wonjang cron add "@every 30m" "다운로드 폴더를 종류별로 정리해줘"
wonjang cron list                 # 등록된 작업 목록
wonjang cron remove 1             # 작업 삭제
wonjang cron run                  # 스케줄러 실행(포그라운드, Ctrl-C로 종료)
```

> ⚠️ `cron run`은 무인 실행이라 도구(셸 명령 포함)를 자동 승인합니다. 신뢰할 수
> 있는 작업만 등록하세요.

**MCP 서버 연동**

외부 [MCP](https://modelcontextprotocol.io) 서버를 연결하면 그 도구들이
`mcp_<서버>_<도구>` 이름으로 에이전트에 자동 등록됩니다. 설정 파일에 추가하세요:

```toml
[[mcp_servers]]
name = "fs"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/work"]
```

```bash
wonjang mcp     # 연결 확인 + 제공 도구 목록
```

**텔레그램 게이트웨이**

휴대폰 등 어디서든 메시지로 원장에게 작업을 시킵니다. [@BotFather](https://t.me/BotFather)로
봇을 만들고 토큰을 환경 변수에 넣으세요.

```bash
export TELEGRAM_BOT_TOKEN=123456:ABC...
wonjang telegram
```

처음엔 허용 목록이 비어 있어, 봇에게 메시지를 보내면 **본인 chat_id**를 알려줍니다.
그 값을 설정에 추가한 뒤 다시 실행하세요:

```toml
telegram_allowed_ids = [123456789]
```

> ⚠️ 게이트웨이는 원격 무인 실행이라 도구(셸 포함)를 자동 승인합니다. 반드시
> 본인 chat_id만 `telegram_allowed_ids`에 등록하세요. 등록되지 않은 사용자의
> 요청은 절대 실행되지 않습니다.

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
| `save_skill` | 재사용 가능한 절차를 스킬로 저장 |
| `list_skills` | 보유 스킬 목록 조회 |
| `read_skill` | 특정 스킬의 전체 절차 읽기 |
| `web_search` | 웹 검색(제목·URL·요약) |
| `web_fetch` | URL 페이지 본문을 텍스트로 가져오기 |
| `spawn_subagent` | 하위 작업을 별도 에이전트에 위임 |
| `spawn_subagents` | 여러 하위 작업을 병렬로 위임 |

저장된 메모리와 스킬은 언제든 확인할 수 있습니다:

```bash
wonjang memory     # 기억하고 있는 사실
wonjang skills     # 익힌 스킬 목록
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
├── skill.rs      # 스킬(절차 기억) 저장소
├── cron.rs       # 예약 작업 스케줄러
├── mcp.rs        # MCP 클라이언트(stdio JSON-RPC)
├── gateway.rs    # 텔레그램 메시징 게이트웨이
├── web.rs        # 웹 검색/가져오기 코어
├── util.rs       # 공용 유틸(동기↔비동기 브리지)
├── ui.rs         # 한국어 우선 터미널 UI
└── tools/        # 로컬 환경 도구
    ├── shell.rs
    ├── fs.rs
    ├── memory.rs
    ├── skill.rs
    ├── web.rs
    ├── subagent.rs
    └── mcp.rs
```

## 로드맵

기능을 점진적으로 추가합니다:

- [x] v0.1 — 에이전트 루프, 제공자 무관 LLM, 셸/파일 도구, 한국어 REPL
- [x] v0.2 — 영속 메모리(remember/recall, 세션 간 사실 유지)
- [x] v0.3 — 세션 저장/이어가기(`--continue`, `wonjang sessions`)
- [x] v0.4 — 스킬(절차 기억) 저장/재사용(save_skill/list_skills/read_skill, `wonjang skills`)
- [x] v0.5 — 웹 검색/페이지 가져오기(web_search/web_fetch, API 키 불필요)
- [x] v0.6 — 예약 작업 스케줄러(`wonjang cron`, 간격 기반 무인 실행)
- [x] v0.7 — 서브에이전트(spawn_subagent/spawn_subagents, 병렬 작업 분할)
- [x] v0.8 — MCP 연동(외부 도구 서버 stdio 연결, `wonjang mcp`)
- [x] v0.9 — 텔레그램 메시징 게이트웨이(`wonjang telegram`, chat_id 화이트리스트)

초기 로드맵을 모두 달성했습니다 🎉 이후에는 안정화·한국 사용자 편의 기능(슬랙/카카오,
음성, 로컬 작업 프리셋 등)을 다듬어 갑니다.
- [ ] 메시징 게이트웨이(텔레그램/슬랙 등)

## 라이선스

MIT

# 원장 에이전트 (wonjangAgent)

[![CI](https://github.com/wonjangcloud9/wonjangAgent/actions/workflows/ci.yml/badge.svg)](https://github.com/wonjangcloud9/wonjangAgent/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/wonjang-agent.svg)](https://www.npmjs.com/package/wonjang-agent)

> 로컬 환경을 다루는 **한국어 우선** 자율 AI 에이전트 — Rust로 작성.

**제공자(provider) 무관 LLM**, **로컬 도구**, **에이전트 루프**, 그리고 한국
사용자를 위한 **한국어 우선 UX**를 갖춘 자율 에이전트입니다.

단일 바이너리로 빠르게 실행되며, 별도 런타임 없이 여러분의 컴퓨터에서 파일과
셸을 직접 다뤄 작업을 수행합니다.

---

## 특징

- 🦀 **단일 러스트 바이너리** — 의존 런타임 없이 빠르게 시작.
- 🇰🇷 **한국어 우선** — 모든 메시지·프롬프트·도움말이 한국어.
- 🔑 **API 키 없이도 OK** — 이미 로그인된 **Claude Code**나 **Codex**를 시작 시
  자동으로 엔진으로 연결합니다(별도 키 불필요). 물론 API 키도 그대로 지원.
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
- 🇰🇷 **작업 프리셋** — "다운로드정리", "오늘커밋" 등 자주 쓰는 작업을 한국어
  이름 하나로 실행. 나만의 프리셋도 추가할 수 있습니다.
- 📓 **옵시디언 연동** — 볼트의 노트를 검색·읽기·기록. 일지, 메모, 할 일을
  한국어 명령 하나로 관리합니다(`일지`, `메모`, `노트검색` 프리셋).
- ⏰ **약속·알림** — "내일 3시 치과 알려줘"처럼 등록하면, 24시간 스케줄러가
  때맞춰 데스크탑 알림을 띄웁니다. "매일 아침 약 먹기"같은 **반복 알림**도 OK
  (`wonjang remind`).
- ☀️ **아침 브리핑** — `브리핑` 프리셋으로 오늘 날짜·날씨·예정된 약속을 한 번에.
- 🧠 **영속 메모리** — 사용자/환경에 대해 배운 사실을 디스크에 저장하고 다음
  세션에 자동으로 불러와 "함께 성장"합니다.
- 📚 **스킬(절차 기억)** — 까다로운 작업을 해결하면 그 방법을 스킬로 저장하고,
  비슷한 일을 만나면 스스로 꺼내 재사용합니다.
- 💾 **세션 저장/이어가기** — 대화가 자동 저장되어 `--continue`로 언제든 이어집니다.
- ⏰ **예약 작업(크론)** — "매일", "30분마다" 같은 일정으로 작업을 무인 실행합니다.
- 🛡 **안전 우선** — 셸 명령은 실행 전 승인을 요청하고, 위험 명령(`rm -rf`, `sudo`,
  `git reset --hard` 등)은 무인 모드(크론·텔레그램)에서 기본 차단합니다.
- 💬 **대화형 REPL + 단발 실행** 모두 지원.

## 설치

### npm (권장)

```bash
npm install -g wonjang-agent
```

설치 시 플랫폼에 맞는 네이티브 바이너리를 자동으로 내려받아 `wonjang` 명령으로
바로 쓸 수 있습니다. 지원: macOS(arm64/x64) · Linux(x64) · Windows(x64).

### 소스 빌드 (cargo)

Rust 툴체인이 필요합니다([rustup.rs](https://rustup.rs)).

```bash
# 저장소에서 바로 설치
cargo install --git https://github.com/wonjangcloud9/wonjangAgent

# 또는 클론 후 빌드
git clone https://github.com/wonjangcloud9/wonjangAgent.git
cd wonjangAgent && cargo build --release   # 결과물: ./target/release/wonjang
```

## 엔진(백엔드) 선택

원장은 시작할 때 사용할 **엔진**을 자동으로 정합니다(`backend = "auto"`):

1. **API 키가 있으면** → OpenAI 호환 API로 자체 도구 루프 실행
2. **없으면** → 이미 로그인된 **Claude Code**(`claude`) 또는 **Codex**(`codex`)에
   연결해 그걸 엔진으로 사용 — **별도 API 키가 필요 없습니다.**

즉, Claude Code나 Codex를 이미 쓰고 있다면 아무 키 설정 없이 바로:

```bash
wonjang "이 폴더 정리해줘"     # → Claude Code/Codex가 실제 작업 수행, 원장이 한국어로 진행
```

CLI 백엔드에서는 사용자의 Claude Code/Codex가 자신의 도구·권한으로 작업합니다.
원장이 `-y`(자동 승인)이면 쓰기 도구(Bash/Edit 등)까지 허용하고, 아니면 읽기 전용으로
제한합니다. 백엔드를 직접 고르려면:

```bash
export WONJANG_BACKEND=claude   # auto | api | claude | codex
```

### API 백엔드 설정

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
| `WONJANG_BACKEND` | 엔진 선택(auto/api/claude/codex) | `auto` |
| `WONJANG_BASE_URL` | OpenAI 호환 베이스 URL | `https://openrouter.ai/api/v1` |
| `WONJANG_MODEL` | 모델 이름 | `anthropic/claude-3.5-sonnet` |
| `WONJANG_API_KEY` | API 키(없으면 `OPENROUTER_API_KEY` → `OPENAI_API_KEY` 폴백) | — |
| `WONJANG_OBSIDIAN_VAULT` | 옵시디언 볼트 경로(노트/일지/메모 기능) | — |

현재 설정/백엔드 확인:

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

**작업 프리셋** 🇰🇷

자주 쓰는 로컬 작업을 한국어 이름 하나로 실행합니다.

```bash
wonjang preset list                    # 사용 가능한 프리셋 목록
wonjang preset run 다운로드정리          # 다운로드 폴더를 종류별로 정리
wonjang preset run 큰파일 "100MB 넘는것만"  # 추가 지시도 가능
wonjang -y preset run 오늘커밋           # -y로 자동 승인
```

빌트인: `다운로드정리` · `바탕화면정리` · `큰파일` · `오늘커밋` · `포트` · `와이파이` ·
`정돈` · `디스크` · `중복파일` · `배터리` · `압축` · `날씨` · `환율` ·
`일지` · `메모` · `노트검색`(옵시디언)

나만의 프리셋은 `~/.config/wonjang/presets.toml`에 추가하세요:

```toml
[[preset]]
name = "백업"
description = "프로젝트를 외장 디스크에 복사"
aliases = ["backup"]
prompt = "현재 프로젝트 폴더를 /Volumes/외장/backup 으로 복사해줘. rsync를 쓰고 진행 전 계획을 보여줘."
```

**REPL 슬래시 명령**

| 명령 | 설명 |
| --- | --- |
| `/help` | 도움말 |
| `/reset` | 대화 기록 초기화 |
| `/exit` | 종료 |

> 💡 **팁:** 구체적으로 요청할수록 잘 동작합니다. "코드 고쳐줘"보다
> "api/handlers.py 47번째 줄 TypeError 고쳐줘"처럼 명확히 말해 주세요.

## 안전장치 🛡

원장은 로컬 환경을 직접 다루므로 안전을 우선합니다.

- **대화형 모드** — 모든 셸 명령은 실행 전 확인을 받습니다. 위험 명령은 빨간 경고와
  함께 더 강하게 확인합니다.
- **무인 모드(크론·텔레그램)** — `rm -rf`, `sudo`, `dd`, `mkfs`, `git reset --hard`,
  포크 폭탄, 파이프-투-셸(`| sh`) 등 되돌리기 어려운 명령을 **기본 차단**합니다.
- 꼭 필요하면 `--allow-dangerous`로 명시적으로 허용할 수 있습니다(권장하지 않음).

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
├── engine.rs     # 백엔드 추상화(API / Claude Code / Codex)
├── cli_backend.rs # Claude Code·Codex CLI 위임
├── agent.rs      # 에이전트 루프(LLM ↔ 도구 왕복, API 백엔드)
├── llm.rs        # OpenAI 호환 LLM 클라이언트
├── config.rs     # 설정 로딩(환경 변수 > 파일 > 기본값)
├── memory.rs     # 영속 메모리(세션 간 사실 유지)
├── notes.rs      # 옵시디언 볼트 연동(검색/읽기/기록)
├── reminders.rs  # 약속·알림(데스크탑 알림)
├── preset.rs     # 작업 프리셋(한국 편의 기능)
├── safety.rs     # 위험 명령 분류기(안전장치)
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

초기 로드맵을 모두 달성했습니다 🎉 이후에는 안정화·한국 사용자 편의 기능을 다듬어 갑니다:

- [x] v0.10 — 작업 프리셋(`wonjang preset`, 한국어 빌트인 + 사용자 정의)
- [x] v0.11 — 위험 명령 안전장치(무인 모드 기본 차단) + 프리셋 13종으로 확장
- [x] v0.12 — CI(fmt/clippy/test) + 멀티플랫폼 릴리스 + npm 배포(`npm i -g wonjang-agent`)
- [x] v0.13 — Claude Code·Codex 백엔드(API 키 없이 시작 시 자동 연결)
- [x] v0.14 — 옵시디언 연동(노트 검색/읽기/기록 + 일지·메모·노트검색 프리셋)
- [x] v0.15 — 약속·알림(`wonjang remind`, 스케줄러가 때맞춰 데스크탑 알림)
- [x] v0.16 — 반복 알림(`--every @daily` 등) + 아침 브리핑 프리셋
- [ ] 노션·디스코드·카카오 연동, 메일/교통(버스·지하철) 비서 기능

## 개발

```bash
cargo test            # 단위 테스트
cargo test -- --ignored   # 네트워크가 필요한 라이브 테스트(web/mcp)
cargo fmt --check     # 포맷 검사
cargo clippy --all-targets -- -D warnings
```

푸시·PR마다 CI(포맷·클리피·테스트)가 돌고, `v*` 태그를 푸시하면 멀티플랫폼
바이너리를 빌드해 GitHub Release에 올리고 npm에 배포합니다. 자세한 릴리스 절차는
[CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

## 라이선스

MIT

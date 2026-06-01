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
- 🗒 **노션 연동** — 워크스페이스를 검색하고 페이지에 기록합니다(`노션검색`,
  `노션저장` 프리셋, `wonjang notion`).
- ⏰ **약속·알림** — "내일 3시 치과 알려줘"처럼 등록하면, 24시간 스케줄러가
  때맞춰 데스크탑 알림을 띄웁니다. "매일 아침 약 먹기"같은 **반복 알림**도 OK
  (`wonjang remind`).
- ✅ **할 일 관리** — `wonjang todo add "장보기"`로 체크리스트 관리. 약속(시각)과
  할 일(목록)을 따로 관리합니다.
- ☀️ **아침 브리핑** — `브리핑` 프리셋으로 오늘 날짜·날씨·약속·할 일을 한 번에.
- 📲 **푸시 알림** — 디스코드/텔레그램/**카카오톡(나에게 보내기)**으로 알림을
  휴대폰에 보냅니다. 약속이 울리면 외출 중에도 받아요(`wonjang notify`).
- 📋 **클립보드 연동** — 복사한 글/링크를 바로 `번역`·`클립요약`·`클립저장`
  (옵시디언) 프리셋으로 처리합니다.
- 📅 **디데이(D-day)** — 수능·기념일·마감 등 중요한 날까지 남은 일수를 챙기고
  아침 브리핑에 보여줍니다(`wonjang dday`).
- 🗂 **현황 대시보드** — `wonjang 현황` 한 줄로 약속·할일·디데이·습관·집중·지출을
  시간대 인사와 함께 한눈에(LLM 없이 즉시). 하루 마무리는 `회고` 프리셋으로.
- 💾 **데이터 백업·복원** — `wonjang 백업`으로 모든 데이터를 한 번에 백업하고,
  `wonjang 복원 <폴더>`로 다른 기기에 그대로 옮깁니다(복원 전 자동 백업).
- 💰 **가계부** — `wonjang 지출 add 8000 식비 점심`으로 지출을 기록하고
  오늘·이번달 합계, 분류별 지출을 봅니다.
- 🔥 **습관 트래커** — `wonjang 습관 add "운동"` / `done`으로 매일 체크하고
  연속 일수(streak)를 챙깁니다(갓생!).
- 🍅 **집중 타이머(뽀모도로)** — `wonjang 집중 25 코딩`으로 타이머+알림을 걸고
  오늘 집중 시간을 누적합니다.
- 🔖 **즐겨찾기 + 빠른 열기** — `wonjang 열기 노션`처럼 자주 가는 사이트·폴더·앱을
  한국어 단축어로 엽니다.
- 🚇 **서울 지하철 실시간** — `wonjang 지하철 강남`으로 역 이름만 입력하면 실시간
  도착정보를 보여줍니다(샘플 키로 바로 동작).
- ☀️ **실시간 날씨** — `wonjang 날씨 [지역]`으로 기온·체감·강수·최저최고를 정확히
  (open-meteo, 키 불필요). 아침 브리핑에도 연동.
- 🌫 **미세먼지** — `wonjang 미세먼지 [지역]`으로 PM10·PM2.5와 환경부 등급
  (좋음/보통/나쁨)을 봅니다(키 불필요).
- 💱 **환율** — `wonjang 환율`로 주요 통화를, `wonjang 환율 100 USD`로 원화 환산을
  실시간으로(키 불필요).
- 🪙 **코인 시세** — `wonjang 코인`으로 업비트 인기 코인을, `wonjang 코인 BTC`로
  특정 코인을 실시간 시세·변동률과 함께(키 불필요).
- 📰 **뉴스** — `wonjang 뉴스 [검색어]`로 최신 헤드라인을(구글뉴스, 키 불필요).
  아침 브리핑에도 연동.
- 🎱 **로또 자동번호** — `wonjang 로또`로 재미로 자동 번호를 뽑습니다.
- 📐 **평수 변환** — `wonjang 평 30`으로 평 ↔ 제곱미터를 양방향으로(부동산 필수).
- 📏 **단위 변환** — `wonjang 변환 100 c`로 온도·무게·길이를 변환합니다(c/f, kg/lb, cm/inch, km/mile).
- ⚖️ **BMI 계산** — `wonjang bmi 175 68`로 체질량지수와 판정·표준체중을(대한비만학회 아시아 기준).
- 🏷️ **할인가 계산** — `wonjang 할인 30000 20 10`으로 (중복) 할인가·절약액·실질 할인율을.
- 🧾 **부가세 계산** — `wonjang 부가세 100000`으로 공급가액·세액을 양방향으로 분리합니다(VAT 10%).
- 📅 **날짜 계산** — `wonjang 날짜 2026-01-01 2026-12-31`로 두 날짜 사이 일수를, `--plus N`으로 N일 후 날짜를.
- ✍️ **글자수 세기** — `wonjang 글자수 "자기소개서 내용"`으로 공백 포함/제외 글자수·단어·줄·바이트를(자소서·SNS).
- 🔡 **한글 초성 추출** — `wonjang 초성 "안녕하세요"`로 초성(ㅇㄴㅎㅅㅇ)을 뽑습니다(초성 퀴즈·검색).
- ⌨️ **한글→영문 타자** — `wonjang 영타 "안녕"`으로 두벌식 키 순서(dkssud)를 알려줍니다.
- 🔁 **영문→한글 복원** — `wonjang 한타 dkssud`로 한/영 전환을 깜빡하고 잘못 친 글자를 "안녕"으로 되살립니다.
- 💴 **한글 금액 표기** — `wonjang 금액 1234567`로 "일금 일백이십삼만사천오백육십칠원정"을(계약서·수표).
- 🧮 **계산기** — `wonjang 계산 "15000 * 1.1 + 3000"`으로 괄호·소수·음수 사칙연산을 바로.
- ⏱️ **시간 계산** — `wonjang 시간 09:00 + 8:30`으로 시·분을 더하고 빼서 근무시간을 합산합니다.
- 🔢 **진법 변환** — `wonjang 진법 255`(또는 `0xFF`)로 2·8·10·16진수를 한 번에 변환합니다.
- 🎂 **만 나이 계산** — `wonjang 나이 1990-03-15`로 만 나이·연 나이·띠·별자리·다음 생일을(만 나이 통일법).
- 💰 **연봉 실수령액** — `wonjang 실수령 3600`으로 4대 보험·소득세 공제 후 월/연 실수령을.
- 💵 **시급·주휴수당** — `wonjang 시급 10030 40`으로 주급·월급을(주 15시간↑ 주휴수당·최저임금 경고).
- 🏦 **대출 상환 계산** — `wonjang 대출 30000 4.5 360`으로 원리금/원금 균등 월 상환액·총이자를.
- 🐷 **예적금 만기 계산** — `wonjang 예금 1000 3.5 12` / `wonjang 적금 50 4.0 24`로 세후 이자·만기 수령액을(이자소득세 15.4%).
- 🍽️ **오늘 뭐 먹지?** — `wonjang 메뉴` 또는 `wonjang 메뉴 중식`으로 식사 메뉴를 추천받습니다.
- 🧾 **더치페이(n빵)** — `wonjang 더치 50000 3`으로 1인당 금액·거스름돈을 정산합니다.
- 🎯 **제비뽑기/추첨** — `wonjang 뽑기 철수 영희 민수`로 당첨자를(또는 `--order`로 순서를) 정합니다.
- 🔔 **시세 알림** — `wonjang 감시 add BTC 110000000`(코인)·`감시 add USD 1400`(환율)처럼
  등록하면, 스케줄러가 지켜보다 목표가 도달 시 휴대폰으로 푸시합니다(24시간 자동 감시).
- 🧠 **영속 메모리** — 사용자/환경에 대해 배운 사실을 디스크에 저장하고 다음
  세션에 자동으로 불러와 "함께 성장"합니다.
- 📚 **스킬(절차 기억)** — 까다로운 작업을 해결하면 그 방법을 스킬로 저장하고,
  비슷한 일을 만나면 스스로 꺼내 재사용합니다.
- 💾 **세션 저장/이어가기** — 대화가 자동 저장되어 `--continue`로 언제든 이어집니다.
- ⏰ **예약 작업(크론)** — "매일", "30분마다" 같은 일정으로 작업을 무인 실행합니다.
- ☀️ **자동 아침 브리핑** — 스케줄러가 매일 정해진 시각에 브리핑을 만들어
  휴대폰(카카오/디스코드/텔레그램)으로 자동 푸시합니다(`WONJANG_BRIEFING_TIME`).
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

> 💡 무엇을 할 수 있는지 한눈에: **`wonjang 도움`**

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

> 버전별 변경 이력은 [CHANGELOG.md](CHANGELOG.md)에서 한눈에 볼 수 있습니다.

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
- [x] v0.17 — 할 일(todo) 관리(`wonjang todo`, 브리핑에 통합)
- [x] v0.18 — 푸시 알림(디스코드/텔레그램, 약속 알림을 휴대폰으로)
- [x] v0.19 — 클립보드 연동(번역·요약·저장 프리셋)
- [x] v0.20 — 디데이(D-day) 관리(chrono 도입, 브리핑 통합)
- [x] v0.21 — 현황 대시보드(`wonjang 현황`, 즉시 한눈에)
- [x] v0.22 — 노션 연동(검색/기록, `wonjang notion` + 프리셋)
- [x] v0.23 — 카카오톡 푸시(나에게 보내기, `WONJANG_KAKAO_TOKEN`)
- [x] v0.24 — 가계부(지출 기록·합계·분류별, `wonjang 지출`)
- [x] v0.25 — 습관 트래커(연속 일수 streak, `wonjang 습관`)
- [x] v0.26 — 집중 타이머/뽀모도로(`wonjang 집중`, 알림 연동)
- [x] v0.27 — 현황 대시보드 강화(습관·집중·지출 통합) + 회고 프리셋
- [x] v0.28 — 자동 아침 브리핑(스케줄러가 매일 정해진 시각에 폰으로 푸시)
- [x] v0.29 — 즐겨찾기 + 빠른 열기(`wonjang 열기`)
- [x] v0.30 — 서울 지하철 실시간 도착(`wonjang 지하철 <역>`)
- [x] v0.31 — 실시간 날씨(`wonjang 날씨`, open-meteo, 키 불필요)
- [x] v0.32 — 미세먼지(`wonjang 미세먼지`, 환경부 등급, 키 불필요)
- [x] v0.33 — 실시간 환율(`wonjang 환율`, 원화 환산, 키 불필요)
- [x] v0.34 — 코인 시세(업비트, `wonjang 코인`, 키 불필요)
- [x] v0.35 — 뉴스 헤드라인(`wonjang 뉴스`, 구글뉴스, 키 불필요)
- [x] v0.36 — 로또 자동번호 추첨(`wonjang 로또`)
- [x] v0.37 — `wonjang 도움` 카테고리별 기능 안내 + 한국어 별칭 정리
- [x] v0.38 — 코인 시세 알림(`wonjang 감시`, 목표가 도달 시 자동 푸시)
- [x] v0.39 — 시세 알림 환율 확장(코인 + 환율 감시)
- [x] v0.40 — 데이터 백업(`wonjang 백업`)
- [x] v0.41 — 데이터 복원(`wonjang 복원`, 복원 전 자동 백업)
- [x] v0.42 — 평수 변환(`wonjang 평`, 평↔㎡)
- [x] v0.43 — 만 나이 계산(`wonjang 나이`, 만 나이 통일법)
- [x] v0.44 — 연봉 실수령액(`wonjang 실수령`, 4대 보험+소득세)
- [x] v0.45 — 대출 상환 계산(`wonjang 대출`, 원리금/원금 균등)
- [x] v0.46 — 예적금 만기 계산(`wonjang 예금`/`적금`, 세후 이자)
- [x] v0.47 — 메뉴 추천(`wonjang 메뉴`, 오늘 뭐 먹지?)
- [x] v0.48 — 더치페이(`wonjang 더치`, n빵 정산)
- [x] v0.49 — 제비뽑기/추첨(`wonjang 뽑기`, 당첨·순서)
- [x] v0.50 — 단위 변환(`wonjang 변환`, 온도/무게/길이)
- [x] v0.51 — BMI 계산(`wonjang bmi`, 아시아 비만 기준)
- [x] v0.52 — 할인가 계산(`wonjang 할인`, 중복 할인)
- [x] v0.53 — 부가세 계산(`wonjang 부가세`, VAT 10% 분리)
- [x] v0.54 — 날짜 계산(`wonjang 날짜`, 일수·N일 후)
- [x] v0.55 — 시급·주휴수당(`wonjang 시급`, 주급·월급)
- [x] v0.57 — 글자수 세기(`wonjang 글자수`, 공백 포함/제외)
- [x] v0.58 — 한글 초성 추출(`wonjang 초성`, 초성 퀴즈)
- [x] v0.59 — 한글 금액 표기(`wonjang 금액`, 계약서·수표)
- [x] v0.60 — 나이에 띠·별자리 추가(`wonjang 나이` 강화)
- [x] v0.61 — 계산기(`wonjang 계산`, 괄호·소수 사칙연산)
- [x] v0.62 — 시간 계산(`wonjang 시간`, 시·분 더하기/빼기)
- [x] v0.63 — 진법 변환(`wonjang 진법`, 2/8/10/16진수)
- [x] v0.64 — 한글→영문 타자(`wonjang 영타`, 두벌식)
- [x] v0.65 — 영문→한글 복원(`wonjang 한타`, 조합 오토마타)
- [ ] 메일·버스 도착 비서 기능

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

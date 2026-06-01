//! 원장 에이전트 — 로컬 환경을 다루는 한국어 우선 자율 AI 에이전트 (Rust).
//!
//! 헤르메스 에이전트(NousResearch/hermes-agent)의 핵심 아이디어를 러스트로
//! 재구성한다: 제공자 무관 LLM, 로컬 도구, 에이전트 루프, 한국어 우선 UX.

mod age;
mod agent;
mod airquality;
mod archive;
mod backup;
mod bike;
mod bmi;
mod bookmarks;
mod briefing;
mod calc;
mod charcount;
mod cli_backend;
mod clipboard;
mod coin;
mod config;
mod congestion;
mod convert;
mod cron;
mod datecalc;
mod ddays;
mod ddoganjip;
mod dedup;
mod deposit;
mod diff;
mod discount;
mod diskusage;
mod dutchpay;
mod engine;
mod exchange;
mod expenses;
mod focus;
mod gateway;
mod geeknews;
mod github;
mod habits;
mod hangul;
mod hash;
mod holidays;
mod jsontool;
mod keyboard;
mod koreannum;
mod llm;
mod loan;
mod lotto;
mod mcp;
mod memory;
mod menu;
mod news;
mod notes;
mod notion;
mod organize;
mod pick;
mod preset;
mod push;
mod pyeong;
mod qr;
mod radix;
mod reminders;
mod rename;
mod safety;
mod salary;
mod search;
mod session;
mod sheet;
mod skill;
mod subway;
mod timecalc;
mod todos;
mod tools;
mod ui;
mod util;
mod vat;
mod wage;
mod watch;
mod weather;
mod web;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use engine::Engine;
use llm::{LlmClient, Message};
use std::io::{self, Write};
use tools::{default_tools, ToolContext};

#[derive(Parser)]
#[command(
    name = "wonjang",
    version,
    about = "원장 — 로컬 환경을 다루는 한국어 우선 AI 에이전트",
    long_about = None
)]
struct Cli {
    /// 한 번에 처리할 요청(생략하면 대화형 모드로 진입).
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,

    /// 셸 명령 등 작업을 자동 승인(확인 없이 실행).
    #[arg(short = 'y', long = "yes")]
    yes: bool,

    /// 위험 명령(rm -rf, sudo 등)도 허용(무인 모드의 기본 차단 해제).
    #[arg(long = "allow-dangerous")]
    allow_dangerous: bool,

    /// 사용할 모델을 일시적으로 지정.
    #[arg(short = 'm', long = "model")]
    model: Option<String>,

    /// 가장 최근 대화를 이어서 진행합니다.
    #[arg(short = 'c', long = "continue")]
    continue_session: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 현재 설정을 보여주고, 없으면 기본 설정 파일을 생성합니다.
    Config,
    /// 에이전트가 기억하고 있는 사실(영속 메모리)을 보여줍니다.
    Memory,
    /// 저장된 대화 세션 목록을 보여줍니다.
    Sessions,
    /// 에이전트가 익힌 스킬(절차 지식) 목록을 보여줍니다.
    Skills,
    /// 약속·알림을 보거나 등록/삭제합니다.
    Remind {
        #[command(subcommand)]
        action: Option<RemindAction>,
    },
    /// 할 일(체크리스트)을 보거나 추가/완료합니다.
    #[command(alias = "할일")]
    Todo {
        #[command(subcommand)]
        action: Option<TodoAction>,
    },
    /// 설정된 채널(디스코드/텔레그램)로 메시지를 푸시합니다.
    Notify {
        /// 보낼 메시지
        #[arg(trailing_var_arg = true)]
        message: Vec<String>,
    },
    /// 디데이(중요한 날까지 남은 일수)를 보거나 등록/삭제합니다.
    Dday {
        #[command(subcommand)]
        action: Option<DdayAction>,
    },
    /// 디스크 용량 분석(큰 파일·폴더 찾기). 예: wonjang 용량 ~/Downloads
    #[command(alias = "용량")]
    Disk {
        /// 분석할 폴더(생략 시 현재 폴더)
        path: Option<String>,
        /// 보여줄 상위 항목 수(기본 10)
        #[arg(long = "개수", default_value_t = 10)]
        top: usize,
    },
    /// 중복 파일 찾기(내용이 같은 파일). 예: wonjang 중복 ~/Downloads
    #[command(alias = "중복")]
    Dedup {
        /// 검사할 폴더(생략 시 현재 폴더)
        path: Option<String>,
        /// 보여줄 묶음 수(기본 10)
        #[arg(long = "개수", default_value_t = 10)]
        top: usize,
    },
    /// 폴더를 종류별로 자동 분류합니다(기본 미리보기). 예: wonjang 정리 ~/Downloads
    #[command(alias = "정리")]
    Organize {
        /// 정리할 폴더
        path: String,
        /// 실제로 이동(미지정 시 미리보기만)
        #[arg(long = "실행")]
        run: bool,
    },
    /// 두 파일 비교(diff). 예: wonjang 비교 old.txt new.txt
    #[command(alias = "비교")]
    Diff {
        /// 원본(이전) 파일
        a: String,
        /// 비교할(이후) 파일
        b: String,
    },
    /// 파일 체크섬(SHA-256/512). 예: wonjang 해시 setup.dmg
    #[command(alias = "해시")]
    Hash {
        /// 파일 경로
        file: String,
        /// 알고리즘(sha256 기본, sha512)
        #[arg(long = "알고리즘", default_value = "sha256")]
        algo: String,
        /// 이 값과 일치하는지 검증
        #[arg(long = "확인")]
        verify: Option<String>,
    },
    /// JSON 파일 검증·정렬·값 추출. 예: wonjang json data.json --키 meta.port
    Json {
        /// JSON 파일 경로
        file: String,
        /// 점 경로로 값 추출(예: meta.port, tags.0)
        #[arg(long = "키")]
        key: Option<String>,
    },
    /// 파일 내용 검색(폴더 안 텍스트에서 단어 찾기). 예: wonjang 찾기 ~/메모 계약
    #[command(alias = "찾기")]
    Search {
        /// 검색할 폴더
        path: String,
        /// 찾을 단어/문구
        query: String,
        /// 결과 상한(기본 50)
        #[arg(long = "개수", default_value_t = 50)]
        max: usize,
    },
    /// 폴더·파일을 zip으로 압축합니다. 예: wonjang 압축 ~/문서
    #[command(alias = "압축")]
    Zip {
        /// 압축할 폴더/파일들
        sources: Vec<String>,
        /// 결과 파일 이름(생략 시 첫 소스 이름.zip)
        #[arg(long = "이름")]
        output: Option<String>,
    },
    /// zip 파일을 풉니다. 예: wonjang 압축풀기 자료.zip
    #[command(alias = "압축풀기")]
    Unzip {
        /// 풀 zip 파일
        file: String,
        /// 풀 폴더(생략 시 zip 이름의 새 폴더)
        dest: Option<String>,
    },
    /// 파일 이름 일괄 변경(특정 문자 치환). 예: wonjang 이름변경 ~/사진 IMG_ 여행_
    #[command(alias = "이름변경")]
    Rename {
        /// 대상 폴더
        path: String,
        /// 찾을 문자열
        find: String,
        /// 바꿀 문자열(빈 문자열이면 삭제)
        #[arg(default_value = "")]
        replace: String,
        /// 실제로 변경(미지정 시 미리보기만)
        #[arg(long = "실행")]
        run: bool,
    },
    /// 풍자 〈또간집〉 선정 맛집을 지역으로 찾습니다. 예: wonjang 또간집 종로
    #[command(alias = "또간집")]
    Ddoganjip {
        /// 검색할 지역(또는 식당 이름). 생략 시 전체 목록
        query: Option<String>,
        /// 식당 추가: 이름. (지역은 --지역 필수)
        #[arg(long = "추가")]
        add: Option<String>,
        /// 추가할 식당의 지역
        #[arg(long = "지역")]
        region: Option<String>,
        /// 추가할 식당의 메모(선택)
        #[arg(long = "메모", default_value = "")]
        note: String,
    },
    /// 엑셀·CSV 파일을 읽어 요약/합계/평균을 냅니다. 예: wonjang 엑셀 매출.csv
    #[command(alias = "엑셀")]
    Excel {
        /// 파일 경로(.csv .tsv .xlsx 등)
        file: String,
        /// 특정 열의 통계(합계·평균·최대·최소). 열 이름 또는 번호
        #[arg(long = "열")]
        column: Option<String>,
        /// 미리볼 행 수(기본 5)
        #[arg(long = "행", default_value_t = 5)]
        rows: usize,
        /// 표 전체를 JSON 배열로 출력(헤더를 키로)
        #[arg(long = "json")]
        json: bool,
    },
    /// 비서 현황을 한눈에 봅니다(약속·할일·디데이·예약작업).
    #[command(alias = "현황")]
    Status,
    /// 원장이 할 수 있는 일을 카테고리별로 안내합니다.
    #[command(alias = "도움")]
    Guide,
    /// 모든 데이터를 백업합니다(약속·할일·가계부·메모리 등).
    #[command(alias = "백업")]
    Backup {
        /// 백업을 저장할 폴더(기본: 홈 디렉터리)
        dest: Option<String>,
    },
    /// 백업 폴더에서 데이터를 복원합니다(복원 전 현재 데이터 자동 백업).
    #[command(alias = "복원")]
    Restore {
        /// 복원할 백업 폴더 경로
        source: String,
    },
    /// 가계부: 지출을 기록하거나 합계를 봅니다.
    #[command(alias = "지출")]
    Expense {
        #[command(subcommand)]
        action: Option<ExpenseAction>,
    },
    /// 습관 트래커: 매일 습관을 체크하고 연속 일수를 봅니다.
    #[command(alias = "습관")]
    Habit {
        #[command(subcommand)]
        action: Option<HabitAction>,
    },
    /// 집중(뽀모도로) 타이머. 예: wonjang 집중 25 코딩 (생략 시 오늘 요약)
    #[command(alias = "집중")]
    Focus {
        /// 집중 시간(분). 생략하면 오늘 집중 요약.
        minutes: Option<i64>,
        /// 무엇에 집중하는지(선택)
        #[arg(trailing_var_arg = true)]
        label: Vec<String>,
    },
    /// 즐겨찾기 관리(사이트/폴더/앱 단축어).
    #[command(alias = "즐겨찾기")]
    Bookmark {
        #[command(subcommand)]
        action: Option<BookmarkAction>,
    },
    /// 즐겨찾기/URL/경로를 기본 프로그램으로 엽니다. 예: wonjang 열기 노션
    #[command(alias = "열기")]
    Open {
        /// 즐겨찾기 이름 또는 URL/경로
        target: String,
    },
    /// 서울 지하철 실시간 도착정보. 예: wonjang 지하철 강남
    #[command(alias = "지하철")]
    Subway {
        /// 역 이름
        station: String,
    },
    /// 실시간 날씨. 예: wonjang 날씨 (생략 시 서울) / wonjang 날씨 부산
    #[command(alias = "날씨")]
    Weather {
        /// 지역 이름(선택)
        #[arg(trailing_var_arg = true)]
        location: Vec<String>,
    },
    /// 미세먼지(대기질). 예: wonjang 미세먼지 (생략 시 서울)
    #[command(alias = "미세먼지")]
    Air {
        /// 지역 이름(선택)
        #[arg(trailing_var_arg = true)]
        location: Vec<String>,
    },
    /// 환율. 예: wonjang 환율 (주요통화) / wonjang 환율 100 USD (환산)
    #[command(alias = "환율")]
    Exchange {
        /// 환산할 금액(선택)
        amount: Option<f64>,
        /// 통화 코드(선택, 예: USD JPY)
        currency: Option<String>,
    },
    /// 코인 시세(업비트). 예: wonjang 코인 (인기) / wonjang 코인 BTC
    #[command(alias = "코인")]
    Coin {
        /// 코인 심볼(선택, 예: BTC)
        symbol: Option<String>,
    },
    /// 뉴스 헤드라인. 예: wonjang 뉴스 (주요) / wonjang 뉴스 경제
    #[command(alias = "뉴스")]
    News {
        /// 검색어(선택)
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },
    /// 로또 자동 번호 추첨. 예: wonjang 로또 (5게임) / wonjang 로또 3
    #[command(alias = "로또")]
    Lotto {
        /// 게임 수(기본 5)
        games: Option<usize>,
    },
    /// 평수 변환(평 ↔ ㎡). 예: wonjang 평 30
    #[command(alias = "평")]
    Pyeong {
        /// 변환할 숫자
        value: f64,
    },
    /// 만 나이 계산(만 나이 통일법 기준). 예: wonjang 나이 1990-03-15
    #[command(alias = "나이")]
    Age {
        /// 생일 (YYYY-MM-DD)
        birth: String,
    },
    /// 연봉 실수령액 계산(4대 보험+소득세). 예: wonjang 실수령 3600
    #[command(alias = "실수령")]
    Salary {
        /// 연봉(만 원 단위). 예: 3600 = 3,600만 원
        manwon: f64,
    },
    /// 대출 상환 계산(원리금/원금 균등). 예: wonjang 대출 30000 4.5 360
    #[command(alias = "대출")]
    Loan {
        /// 원금(만 원 단위). 예: 30000 = 3억
        manwon: f64,
        /// 연이율(%). 예: 4.5
        rate: f64,
        /// 상환 개월 수. 예: 360 = 30년
        months: u32,
    },
    /// 정기예금 만기 계산(세후). 예: wonjang 예금 1000 3.5 12
    #[command(alias = "예금")]
    Deposit {
        /// 예치 원금(만 원 단위). 예: 1000 = 1,000만 원
        manwon: f64,
        /// 연이율(%). 예: 3.5
        rate: f64,
        /// 예치 개월 수. 예: 12
        months: u32,
    },
    /// 정기적금 만기 계산(세후). 예: wonjang 적금 50 4.0 24
    #[command(alias = "적금")]
    Savings {
        /// 월 납입액(만 원 단위). 예: 50 = 50만 원
        manwon: f64,
        /// 연이율(%). 예: 4.0
        rate: f64,
        /// 납입 개월 수. 예: 24
        months: u32,
    },
    /// 오늘 뭐 먹지? 메뉴 추천. 예: wonjang 메뉴 / wonjang 메뉴 중식
    #[command(alias = "메뉴")]
    Menu {
        /// 카테고리(한식/중식/일식/양식/분식/야식). 생략 시 전체에서 추천
        category: Option<String>,
    },
    /// 더치페이(n빵) 정산. 예: wonjang 더치 50000 3
    #[command(alias = "더치")]
    Dutch {
        /// 총액(원)
        total: i64,
        /// 인원수
        people: i64,
        /// 올림 단위(원, 기본 100)
        #[arg(default_value_t = 100)]
        unit: i64,
    },
    /// 단위 변환(온도/무게/길이). 예: wonjang 변환 100 c
    #[command(alias = "변환")]
    Convert {
        /// 값
        value: f64,
        /// 단위(c/f, kg/lb, cm/inch, km/mile)
        unit: String,
    },
    /// BMI 계산(아시아 기준 판정). 예: wonjang bmi 175 68
    Bmi {
        /// 키(cm)
        height: f64,
        /// 몸무게(kg)
        weight: f64,
    },
    /// 할인가 계산(중복 할인 가능). 예: wonjang 할인 30000 20 10
    #[command(alias = "할인")]
    Discount {
        /// 원가(원)
        price: f64,
        /// 할인율(%) 목록. 여러 개면 순차 적용. 예: 20 10
        rates: Vec<f64>,
    },
    /// 부가세(VAT 10%) 계산. 예: wonjang 부가세 100000
    #[command(alias = "부가세")]
    Vat {
        /// 금액(원)
        amount: f64,
    },
    /// 글자수 세기(공백 포함/제외). 예: wonjang 글자수 "자기소개서 내용"
    #[command(alias = "글자수")]
    Chars {
        /// 셀 텍스트(여러 단어면 공백으로 이어 붙여 셈)
        text: Vec<String>,
    },
    /// 한글 초성 추출(초성 퀴즈·검색). 예: wonjang 초성 "안녕하세요"
    #[command(alias = "초성")]
    Choseong {
        /// 초성을 뽑을 텍스트
        text: Vec<String>,
    },
    /// 한글 → 영문 타자 변환. 예: wonjang 영타 "안녕"
    #[command(alias = "영타")]
    Keystroke {
        /// 변환할 한글 텍스트
        text: Vec<String>,
    },
    /// 영문 → 한글 복원(잘못 친 한글). 예: wonjang 한타 dkssud
    #[command(alias = "한타")]
    Hanstroke {
        /// 복원할 영문 텍스트(예: dkssud)
        text: Vec<String>,
    },
    /// 숫자 → 한글 금액(계약서·수표). 예: wonjang 금액 1234567
    #[command(alias = "금액")]
    Amount {
        /// 금액(원)
        value: u64,
    },
    /// 사칙연산 계산기. 예: wonjang 계산 "15000 * 1.1 + 3000"
    #[command(alias = "계산")]
    Calc {
        /// 계산할 식(괄호·소수·음수 가능)
        expr: Vec<String>,
    },
    /// 시간 계산(시·분 더하기/빼기). 예: wonjang 시간 09:00 + 8:30
    #[command(alias = "시간")]
    Time {
        /// 시간(H:MM)과 부호(+/-) 항목들
        items: Vec<String>,
    },
    /// 진법 변환(2/8/10/16). 예: wonjang 진법 255 / wonjang 진법 0xFF
    #[command(alias = "진법")]
    Radix {
        /// 숫자(접두사 0x/0o/0b로 진법 자동 인식)
        value: String,
    },
    /// 시급·주휴수당 계산(주급/월급). 예: wonjang 시급 10030 40
    #[command(alias = "시급")]
    Wage {
        /// 시급(원)
        hourly: f64,
        /// 주당 근로시간
        weekly_hours: f64,
    },
    /// 한국 공휴일 조회(설날·추석 포함). 예: wonjang 공휴일 [2026]
    #[command(alias = "공휴일")]
    Holiday {
        /// 연도(생략 시 올해)
        year: Option<i32>,
    },
    /// 서울 실시간 혼잡도 조회. 예: wonjang 혼잡도 강남역
    #[command(alias = "혼잡도")]
    Congestion {
        /// 지역 이름(명소·상권·역 등). 예: 강남역, 홍대, 여의도
        area: String,
    },
    /// 긱뉴스(개발·기술·스타트업) 최신글. 예: wonjang 긱뉴스
    #[command(alias = "긱뉴스")]
    Geeknews {
        /// 보여줄 개수(기본 10)
        count: Option<usize>,
    },
    /// GitHub 저장소 정보(별·이슈·릴리스). 예: wonjang 깃헙 rust-lang/rust
    #[command(alias = "깃헙")]
    Github {
        /// owner/repo
        slug: String,
    },
    /// QR 코드를 터미널에 생성합니다. 예: wonjang qr https://example.com
    Qr {
        /// QR로 만들 텍스트/URL
        text: Vec<String>,
        /// 와이파이 QR: SSID(비밀번호는 --비번)
        #[arg(long = "wifi")]
        wifi: Option<String>,
        /// 와이파이 비밀번호
        #[arg(long = "비번", default_value = "")]
        password: String,
    },
    /// 서울 따릉이 실시간(남은 자전거·거치대). 예: wonjang 따릉이 강남역
    #[command(alias = "따릉이")]
    Bike {
        /// 대여소 이름 검색어(예: 강남역, 망원역)
        query: Option<String>,
    },
    /// 날짜 계산(두 날짜 사이 일수 / N일 후). 예: wonjang 날짜 2026-01-01 2026-12-31
    #[command(alias = "날짜")]
    Date {
        /// 기준 날짜(YYYY-MM-DD). 생략 시 오늘
        from: Option<String>,
        /// 비교 날짜(YYYY-MM-DD) — 두 날짜 사이 일수
        to: Option<String>,
        /// 기준 날짜에 N일 더하기(음수면 빼기)
        #[arg(long, allow_hyphen_values = true)]
        plus: Option<i64>,
    },
    /// 제비뽑기/랜덤 추첨. 예: wonjang 뽑기 철수 영희 민수
    #[command(alias = "뽑기")]
    Pick {
        /// 뽑을 인원/개수(기본 1)
        #[arg(short = 'n', long = "count", default_value_t = 1)]
        count: usize,
        /// 순서 섞기(당첨 대신 전체 순서를 보여줌)
        #[arg(short = 'o', long = "order")]
        order: bool,
        /// 후보 항목들(이름·메뉴 등)
        items: Vec<String>,
    },
    /// 코인 시세 알림(목표가 도달 시 푸시). 스케줄러가 켜져 있어야 동작.
    #[command(alias = "감시")]
    Watch {
        #[command(subcommand)]
        action: Option<WatchAction>,
    },
    /// 노션 워크스페이스를 검색하거나 페이지에 기록합니다.
    Notion {
        #[command(subcommand)]
        action: NotionAction,
    },
    /// 설정된 MCP 서버에 연결해 제공 도구 목록을 보여줍니다.
    Mcp,
    /// 텔레그램 봇 게이트웨이를 실행합니다(메시지로 원장에게 작업 지시).
    Telegram,
    /// 자주 쓰는 작업 프리셋을 보거나 실행합니다.
    Preset {
        #[command(subcommand)]
        action: PresetAction,
    },
    /// 예약 작업(크론)을 관리하고 실행합니다.
    Cron {
        #[command(subcommand)]
        action: CronAction,
    },
}

#[derive(Subcommand)]
enum PresetAction {
    /// 사용 가능한 프리셋 목록을 보여줍니다.
    List,
    /// 프리셋을 실행합니다. 예: wonjang preset run 다운로드정리
    Run {
        /// 프리셋 이름 또는 별칭
        name: String,
        /// 추가 지시(선택)
        #[arg(trailing_var_arg = true)]
        extra: Vec<String>,
    },
}

#[derive(Subcommand)]
enum WatchAction {
    /// 등록된 시세 알림 목록(기본).
    List,
    /// 알림 추가. 예: wonjang 감시 add BTC 110000000 (목표가 도달 시 알림)
    Add {
        /// 코인 심볼(예: BTC)
        symbol: String,
        /// 목표가(원)
        target: f64,
    },
    /// id로 알림 삭제.
    Remove {
        /// 삭제할 알림 id
        id: u64,
    },
}

#[derive(Subcommand)]
enum BookmarkAction {
    /// 즐겨찾기 목록(기본).
    List,
    /// 즐겨찾기 추가. 예: wonjang 즐겨찾기 add 노션 https://notion.so
    Add {
        /// 단축 이름
        name: String,
        /// 대상(URL/경로/앱)
        target: String,
    },
    /// id로 즐겨찾기 삭제.
    Remove {
        /// 삭제할 즐겨찾기 id
        id: u64,
    },
}

#[derive(Subcommand)]
enum HabitAction {
    /// 습관 목록(오늘 여부 + 연속 일수). 기본.
    List,
    /// 습관 추가. 예: wonjang 습관 add "운동"
    Add {
        /// 습관 이름
        name: String,
    },
    /// 오늘 습관 완료. 예: wonjang 습관 done 운동
    Done {
        /// 습관 이름 또는 id
        habit: String,
    },
    /// id로 습관 삭제.
    Remove {
        /// 삭제할 습관 id
        id: u64,
    },
}

#[derive(Subcommand)]
enum ExpenseAction {
    /// 오늘 지출 추가. 예: wonjang 지출 add 8000 식비 점심
    Add {
        /// 금액(원)
        amount: i64,
        /// 분류(식비/교통/배달 등)
        category: String,
        /// 메모(선택)
        #[arg(trailing_var_arg = true)]
        note: Vec<String>,
    },
    /// 이번 달 분류별 지출.
    Month,
    /// id로 지출 기록 삭제.
    Remove {
        /// 삭제할 지출 id
        id: u64,
    },
}

#[derive(Subcommand)]
enum NotionAction {
    /// 노션 검색. 예: wonjang notion search "회의록"
    Search {
        /// 검색어
        query: String,
    },
    /// 페이지에 기록. 예: wonjang notion append <page_id> "오늘 메모"
    Append {
        /// 대상 페이지 id
        page_id: String,
        /// 덧붙일 텍스트
        text: String,
    },
}

#[derive(Subcommand)]
enum DdayAction {
    /// 디데이 목록(기본).
    List,
    /// 디데이 추가. 예: wonjang dday add "수능" 2026-11-19
    Add {
        /// 디데이 이름(여러 단어면 따옴표)
        label: String,
        /// 목표 날짜 YYYY-MM-DD
        date: String,
    },
    /// id로 디데이 삭제.
    Remove {
        /// 삭제할 디데이 id
        id: u64,
    },
}

#[derive(Subcommand)]
enum TodoAction {
    /// 할 일 목록(기본).
    List,
    /// 할 일 추가. 예: wonjang todo add "장보기"
    Add {
        /// 할 일 내용(여러 단어면 따옴표)
        text: String,
    },
    /// id로 할 일 완료 처리.
    Done {
        /// 완료할 할 일 id
        id: u64,
    },
    /// id로 할 일 삭제.
    Remove {
        /// 삭제할 할 일 id
        id: u64,
    },
    /// 완료된 할 일을 모두 정리.
    Clear,
}

#[derive(Subcommand)]
enum RemindAction {
    /// 예정된 알림 목록(기본).
    List,
    /// 알림 추가. 예: wonjang remind add 30 "물 마시기" --every @daily
    Add {
        /// 지금부터 N분 뒤(첫 알림)
        minutes: i64,
        /// 알림 제목(여러 단어면 따옴표로 감싸기)
        title: String,
        /// 반복 주기(@daily, @weekly, @hourly, 1d, 12h 등)
        #[arg(long = "every")]
        every: Option<String>,
    },
    /// id로 알림 삭제.
    Remove {
        /// 삭제할 알림 id
        id: u64,
    },
}

#[derive(Subcommand)]
enum CronAction {
    /// 예약 작업을 추가합니다. 예: wonjang cron add "@daily" "어제 받은 메일 요약해줘"
    Add {
        /// 스케줄(@hourly, @daily, @every 30m, 2h 등)
        schedule: String,
        /// 실행할 요청
        prompt: String,
    },
    /// 등록된 예약 작업 목록을 보여줍니다.
    List,
    /// id로 예약 작업을 삭제합니다.
    Remove {
        /// 삭제할 작업 id
        id: u64,
    },
    /// 스케줄러를 실행합니다(포그라운드 데몬). 종료는 Ctrl-C.
    Run,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        ui::error(&format!("{e:#}"));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut cfg = Config::load()?;
    if let Some(m) = &cli.model {
        cfg.model = m.clone();
    }

    // LLM이 필요 없는 서브커맨드 처리.
    match &cli.command {
        Some(Commands::Config) => return cmd_config(&cfg),
        Some(Commands::Memory) => return cmd_memory(),
        Some(Commands::Sessions) => return cmd_sessions(),
        Some(Commands::Skills) => return cmd_skills(),
        Some(Commands::Remind { action }) => return cmd_remind(action),
        Some(Commands::Todo { action }) => return cmd_todo(action),
        Some(Commands::Notify { message }) => return cmd_notify(&cfg, message),
        Some(Commands::Dday { action }) => return cmd_dday(action),
        Some(Commands::Excel {
            file,
            column,
            rows,
            json,
        }) => return cmd_excel(file, column.as_deref(), *rows, *json),
        Some(Commands::Ddoganjip {
            query,
            add,
            region,
            note,
        }) => return cmd_ddoganjip(query.as_deref(), add.as_deref(), region.as_deref(), note),
        Some(Commands::Disk { path, top }) => return cmd_disk(path.as_deref(), *top),
        Some(Commands::Dedup { path, top }) => return cmd_dedup(path.as_deref(), *top),
        Some(Commands::Organize { path, run }) => return cmd_organize(path, *run),
        Some(Commands::Diff { a, b }) => return cmd_diff(a, b),
        Some(Commands::Hash { file, algo, verify }) => {
            return cmd_hash(file, algo, verify.as_deref())
        }
        Some(Commands::Json { file, key }) => return cmd_json(file, key.as_deref()),
        Some(Commands::Search { path, query, max }) => return cmd_search(path, query, *max),
        Some(Commands::Zip { sources, output }) => return cmd_zip(sources, output.as_deref()),
        Some(Commands::Unzip { file, dest }) => return cmd_unzip(file, dest.as_deref()),
        Some(Commands::Rename {
            path,
            find,
            replace,
            run,
        }) => return cmd_rename(path, find, replace, *run),
        Some(Commands::Status) => return cmd_status(),
        Some(Commands::Guide) => return cmd_guide(),
        Some(Commands::Backup { dest }) => return cmd_backup(dest),
        Some(Commands::Restore { source }) => return cmd_restore(source),
        Some(Commands::Expense { action }) => return cmd_expense(action),
        Some(Commands::Habit { action }) => return cmd_habit(action),
        Some(Commands::Focus { minutes, label }) => return cmd_focus(*minutes, label),
        Some(Commands::Bookmark { action }) => return cmd_bookmark(action),
        Some(Commands::Open { target }) => return cmd_open(target),
        Some(Commands::Subway { station }) => return cmd_subway(&cfg, station),
        Some(Commands::Weather { location }) => return cmd_weather(location),
        Some(Commands::Air { location }) => return cmd_air(location),
        Some(Commands::Exchange { amount, currency }) => return cmd_exchange(*amount, currency),
        Some(Commands::Coin { symbol }) => return cmd_coin(symbol),
        Some(Commands::News { query }) => return cmd_news(query),
        Some(Commands::Lotto { games }) => return cmd_lotto(*games),
        Some(Commands::Pyeong { value }) => return cmd_pyeong(*value),
        Some(Commands::Age { birth }) => return cmd_age(birth),
        Some(Commands::Salary { manwon }) => return cmd_salary(*manwon),
        Some(Commands::Loan {
            manwon,
            rate,
            months,
        }) => return cmd_loan(*manwon, *rate, *months),
        Some(Commands::Deposit {
            manwon,
            rate,
            months,
        }) => return cmd_deposit(*manwon, *rate, *months, false),
        Some(Commands::Savings {
            manwon,
            rate,
            months,
        }) => return cmd_deposit(*manwon, *rate, *months, true),
        Some(Commands::Menu { category }) => return cmd_menu(category.as_deref()),
        Some(Commands::Dutch {
            total,
            people,
            unit,
        }) => return cmd_dutch(*total, *people, *unit),
        Some(Commands::Convert { value, unit }) => return cmd_convert(*value, unit),
        Some(Commands::Bmi { height, weight }) => return cmd_bmi(*height, *weight),
        Some(Commands::Discount { price, rates }) => return cmd_discount(*price, rates),
        Some(Commands::Vat { amount }) => return cmd_vat(*amount),
        Some(Commands::Chars { text }) => return cmd_chars(text),
        Some(Commands::Choseong { text }) => return cmd_choseong(text),
        Some(Commands::Keystroke { text }) => return cmd_keystroke(text),
        Some(Commands::Hanstroke { text }) => return cmd_hanstroke(text),
        Some(Commands::Amount { value }) => return cmd_amount(*value),
        Some(Commands::Calc { expr }) => return cmd_calc(expr),
        Some(Commands::Time { items }) => return cmd_time(items),
        Some(Commands::Radix { value }) => return cmd_radix(value),
        Some(Commands::Wage {
            hourly,
            weekly_hours,
        }) => return cmd_wage(*hourly, *weekly_hours),
        Some(Commands::Holiday { year }) => return cmd_holiday(*year),
        Some(Commands::Congestion { area }) => return cmd_congestion(&cfg, area),
        Some(Commands::Geeknews { count }) => return cmd_geeknews(*count),
        Some(Commands::Qr {
            text,
            wifi,
            password,
        }) => return cmd_qr(text, wifi.as_deref(), password),
        Some(Commands::Github { slug }) => return cmd_github(slug),
        Some(Commands::Bike { query }) => return cmd_bike(&cfg, query.as_deref()),
        Some(Commands::Date { from, to, plus }) => {
            return cmd_date(from.as_deref(), to.as_deref(), *plus)
        }
        Some(Commands::Pick {
            count,
            order,
            items,
        }) => return cmd_pick(items, *count, *order),
        Some(Commands::Watch { action }) => return cmd_watch(action),
        Some(Commands::Notion { action }) => return cmd_notion(&cfg, action),
        Some(Commands::Mcp) => return cmd_mcp(&cfg),
        Some(Commands::Telegram) => {} // LLM 필요 — 아래에서 처리.
        Some(Commands::Preset { action }) => match action {
            PresetAction::List => return cmd_preset_list(),
            PresetAction::Run { name, .. } => {
                // 존재 검증을 API 키 검사보다 먼저(오타 시 명확한 안내).
                if preset::find(name).is_none() {
                    ui::error(&format!(
                        "'{name}' 프리셋을 찾을 수 없습니다. 목록: wonjang preset list"
                    ));
                    std::process::exit(1);
                }
            } // 유효하면 LLM 경로에서 실행.
        },
        Some(Commands::Cron { action }) => match action {
            CronAction::Add { schedule, prompt } => return cmd_cron_add(schedule, prompt),
            CronAction::List => return cmd_cron_list(),
            CronAction::Remove { id } => return cmd_cron_remove(*id),
            CronAction::Run => {} // 아래에서 클라이언트 구성 후 데몬 실행.
        },
        None => {}
    }

    // 백엔드 결정: API 키가 있으면 api, 없으면 Claude Code/Codex CLI 자동 연결.
    let backend = engine::resolve(&cfg)?;
    let eng = build_engine(backend, &cfg);
    ui::info(&format!("백엔드: {}", eng.label(&cfg)));

    let ctx = ToolContext {
        auto_approve: cli.yes,
        allow_dangerous: cli.allow_dangerous,
    };

    // 크론 데몬.
    if let Some(Commands::Cron {
        action: CronAction::Run,
    }) = &cli.command
    {
        return cmd_cron_run(&eng, &cfg).await;
    }

    // 텔레그램 게이트웨이.
    if let Some(Commands::Telegram) = &cli.command {
        return gateway::run_telegram(&eng, &cfg).await;
    }

    // 세션: 이어가기(--continue) 또는 새 세션.
    let (sess, mut messages) = if cli.continue_session {
        let (s, msgs) = session::Session::latest_or_new()?;
        if !msgs.is_empty() {
            ui::info(&format!("이전 대화를 이어갑니다(메시지 {}개).", msgs.len()));
        }
        (s, msgs)
    } else {
        (session::Session::new()?, Vec::new())
    };

    // 새 세션이면 영속 메모리 + 보유 스킬 목록을 시스템 프롬프트에 주입.
    if messages.is_empty() {
        let mem = memory::Memory::load()?;
        let skills = skill::SkillStore::load()?;
        messages.push(Message::system(agent::system_prompt(
            mem.prompt_block(),
            skills.prompt_block(),
        )));
    }

    // 프리셋 실행(단발 모드로 처리).
    let preset_prompt = if let Some(Commands::Preset {
        action: PresetAction::Run { name, extra },
    }) = &cli.command
    {
        match preset::find(name) {
            Some(p) => {
                ui::note(&format!("프리셋 실행: {} — {}", p.name, p.description));
                let mut prompt = p.prompt;
                if !extra.is_empty() {
                    prompt.push_str("\n\n추가 지시: ");
                    prompt.push_str(&extra.join(" "));
                }
                Some(prompt)
            }
            None => {
                ui::error(&format!(
                    "'{name}' 프리셋을 찾을 수 없습니다. 목록: wonjang preset list"
                ));
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // 단발 실행 모드(직접 입력 또는 프리셋).
    let one_shot = preset_prompt.unwrap_or_else(|| cli.prompt.join(" "));
    if !one_shot.trim().is_empty() {
        messages.push(Message::user(one_shot));
        let answer = eng.run(&cfg, &ctx, &mut messages).await?;
        agent::print_answer(&answer);
        sess.save(&messages).ok();
        return Ok(());
    }

    // 대화형 REPL 모드.
    repl(&eng, &cfg, &ctx, &mut messages, &sess).await
}

/// 백엔드에 맞는 엔진을 구성한다.
fn build_engine(backend: engine::Backend, cfg: &Config) -> Engine {
    match backend {
        engine::Backend::Api => {
            let client =
                LlmClient::new(cfg.base_url.clone(), cfg.api_key.clone(), cfg.model.clone());
            let mut tools = default_tools();
            // 설정된 MCP 서버에 연결해 외부 도구를 등록한다(실패해도 계속 진행).
            for srv in &cfg.mcp_servers {
                match mcp::McpClient::connect(&srv.name, &srv.command, &srv.args, &srv.env) {
                    Ok(c) => {
                        let n = c.tools.len();
                        tools.extend(tools::mcp::tools_from_client(std::sync::Arc::new(c)));
                        ui::info(&format!("MCP '{}' 연결됨 — 도구 {n}개", srv.name));
                    }
                    Err(e) => ui::error(&format!("MCP '{}' 연결 실패: {e:#}", srv.name)),
                }
            }
            Engine::Api { client, tools }
        }
        engine::Backend::Claude => Engine::Cli(cli_backend::CliKind::Claude),
        engine::Backend::Codex => Engine::Cli(cli_backend::CliKind::Codex),
    }
}

/// 대화형 모드.
async fn repl(
    eng: &Engine,
    cfg: &Config,
    ctx: &ToolContext,
    messages: &mut Vec<Message>,
    sess: &session::Session,
) -> Result<()> {
    ui::banner(&eng.label(cfg));

    loop {
        print!("{}", ui::prompt());
        io::stdout().flush()?;

        let mut line = String::new();
        let n = io::stdin().read_line(&mut line)?;
        if n == 0 {
            // EOF(Ctrl-D)
            println!();
            ui::info("안녕히 가세요. 👋");
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        // 슬래시 명령.
        match input {
            "/exit" | "/quit" | "/종료" => {
                ui::info("안녕히 가세요. 👋");
                break;
            }
            "/help" | "/도움말" => {
                print_help();
                continue;
            }
            "/reset" | "/초기화" => {
                messages.truncate(1); // 시스템 프롬프트만 남김.
                sess.save(messages).ok();
                ui::info("대화 기록을 초기화했습니다.");
                continue;
            }
            _ => {}
        }

        messages.push(Message::user(input.to_string()));
        match eng.run(cfg, ctx, messages).await {
            Ok(answer) => agent::print_answer(&answer),
            Err(e) => ui::error(&format!("{e:#}")),
        }
        // 매 턴 후 세션을 저장(중간에 종료해도 이어가기 가능).
        sess.save(messages).ok();
    }
    Ok(())
}

fn print_help() {
    ui::info(
        "사용 가능한 명령:\n  \
         /help     이 도움말\n  \
         /reset    대화 기록 초기화\n  \
         /exit     종료\n\n\
         대화는 자동 저장됩니다. 다음에 `wonjang --continue`로 이어갈 수 있어요.\n\
         그 외에는 무엇이든 한국어로 요청하세요. 예) '이 폴더 파일 정리해줘', \
         'git 상태 알려줘', 'README 초안 작성해줘'",
    );
}

fn cmd_config(cfg: &Config) -> Result<()> {
    let path = config::config_path()?;
    if !path.exists() {
        let saved = cfg.save()?;
        ui::note(&format!(
            "기본 설정 파일을 생성했습니다: {}",
            saved.display()
        ));
    }
    println!("현재 설정:");
    println!("  설정 파일 : {}", path.display());
    let resolved = match engine::resolve(cfg) {
        Ok(b) => format!("{b:?}"),
        Err(_) => "없음(키도 CLI도 미발견)".to_string(),
    };
    println!("  backend   : {} → 사용: {resolved}", cfg.backend);
    println!("  base_url  : {}", cfg.base_url);
    println!("  model     : {}", cfg.model);
    println!(
        "  api_key   : {}",
        if cfg.api_key.is_empty() {
            "(없음 — 환경 변수로 설정 필요)".to_string()
        } else {
            "(설정됨, 환경 변수)".to_string()
        }
    );
    println!("  max_steps : {}", cfg.max_steps);
    println!("  MCP 서버  : {}개", cfg.mcp_servers.len());
    let channels = push::configured_channels(cfg);
    println!(
        "  푸시 채널  : {}",
        if channels.is_empty() {
            "(없음)".to_string()
        } else {
            channels.join(", ")
        }
    );
    println!(
        "  옵시디언  : {}",
        if cfg.obsidian_vault.is_empty() {
            "(미설정)"
        } else {
            &cfg.obsidian_vault
        }
    );
    println!(
        "  노션      : {}",
        if cfg.notion_token.is_empty() {
            "(토큰 미설정)"
        } else {
            "(토큰 설정됨)"
        }
    );
    println!(
        "  자동 브리핑: {}",
        if cfg.briefing_time.is_empty() {
            "(꺼짐)".to_string()
        } else {
            format!("매일 {} (cron run 필요)", cfg.briefing_time)
        }
    );
    println!(
        "  텔레그램  : {} / 허용 chat_id {}개",
        if cfg.telegram_token.is_empty() {
            "토큰 없음"
        } else {
            "토큰 설정됨"
        },
        cfg.telegram_allowed_ids.len()
    );
    let seoul = if cfg.seoul_api_key == "sample" || cfg.seoul_api_key.is_empty() {
        "sample (예시 데이터) — 무료 키 권장"
    } else {
        "설정됨 ✔"
    };
    println!("  서울 K-API: {seoul}");

    println!();
    println!("  🔑 무료 키로 더 많은 한국 기능 켜기:");
    println!("     • 서울 열린데이터(data.seoul.go.kr): 지하철(전체)·혼잡도(임의지역)·따릉이");
    println!("       → config.toml에 seoul_api_key = \"발급키\"");
    println!("     • 공공데이터포털(data.go.kr): 주유소 최저가·응급실·버스도착·약국 등(예정)");
    println!("     • 네이버 개발자(developers.naver.com): 쇼핑 최저가·블로그·책 검색(예정)");
    ui::info("\nAPI 키·토큰 등 비밀값은 파일에 저장하지 않습니다(서울 키처럼 비밀이 아닌 값만 파일 저장).");
    Ok(())
}

fn cmd_sessions() -> Result<()> {
    let items = session::list()?;
    if items.is_empty() {
        ui::info("저장된 세션이 없습니다. 대화를 시작하면 자동으로 저장됩니다.");
        return Ok(());
    }
    println!("저장된 세션(최신순):\n");
    for (i, (path, preview, count)) in items.iter().enumerate() {
        let marker = if i == 0 { "→" } else { " " };
        println!("  {marker} {preview}  ({count}개 메시지)");
        ui::info(&format!("     {}", path.display()));
    }
    println!();
    ui::info("가장 최근 세션을 이어가려면: wonjang --continue");
    Ok(())
}

fn cmd_preset_list() -> Result<()> {
    let presets = preset::load_all();
    println!("사용 가능한 프리셋({}개):\n", presets.len());
    for p in &presets {
        let alias = if p.aliases.is_empty() {
            String::new()
        } else {
            format!("  (별칭: {})", p.aliases.join(", "))
        };
        println!("  • {}{}", p.name, alias);
        ui::info(&format!("     {}", p.description));
    }
    println!();
    ui::info("실행: wonjang preset run <이름> [추가 지시]");
    ui::info(&format!(
        "나만의 프리셋 추가: {}",
        preset::user_presets_path()?.display()
    ));
    Ok(())
}

fn cmd_mcp(cfg: &Config) -> Result<()> {
    if cfg.mcp_servers.is_empty() {
        ui::info("설정된 MCP 서버가 없습니다.");
        println!(
            "\n설정 파일({})에 다음과 같이 추가하세요:\n",
            config::config_path()?.display()
        );
        println!(
            r#"[[mcp_servers]]
name = "fs"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/dir"]"#
        );
        return Ok(());
    }
    for srv in &cfg.mcp_servers {
        println!("• {} ({} {})", srv.name, srv.command, srv.args.join(" "));
        match mcp::McpClient::connect(&srv.name, &srv.command, &srv.args, &srv.env) {
            Ok(c) => {
                if c.tools.is_empty() {
                    ui::info("    (제공 도구 없음)");
                }
                for t in &c.tools {
                    let desc = t.description.lines().next().unwrap_or("");
                    println!("    - {} : {}", t.name, desc);
                }
            }
            Err(e) => ui::error(&format!("    연결 실패: {e:#}")),
        }
    }
    Ok(())
}

fn cmd_cron_add(schedule: &str, prompt: &str) -> Result<()> {
    let mut store = cron::CronStore::load()?;
    let id = store.add(schedule, prompt)?;
    ui::note(&format!("예약 작업 #{id} 등록: [{schedule}] {prompt}"));
    ui::info("스케줄러를 켜려면: wonjang cron run");
    Ok(())
}

fn cmd_cron_list() -> Result<()> {
    let store = cron::CronStore::load()?;
    if store.tasks.is_empty() {
        ui::info("등록된 예약 작업이 없습니다. 예: wonjang cron add \"@daily\" \"할 일 요약해줘\"");
        return Ok(());
    }
    println!("예약 작업 목록:\n");
    for t in &store.tasks {
        let state = if t.enabled { "켜짐" } else { "꺼짐" };
        println!("  #{}  [{}]  ({})", t.id, t.schedule, state);
        println!("      {}", t.prompt);
    }
    println!();
    ui::info("스케줄러 실행: wonjang cron run   |   삭제: wonjang cron remove <id>");
    Ok(())
}

fn cmd_cron_remove(id: u64) -> Result<()> {
    let mut store = cron::CronStore::load()?;
    if store.remove(id)? {
        ui::note(&format!("예약 작업 #{id}을(를) 삭제했습니다."));
    } else {
        ui::error(&format!("작업 #{id}을(를) 찾을 수 없습니다."));
    }
    Ok(())
}

/// 크론 데몬 — 포그라운드에서 주기적으로 due 작업을 실행한다.
async fn cmd_cron_run(eng: &Engine, cfg: &Config) -> Result<()> {
    let store = cron::CronStore::load()?;
    ui::note(&format!(
        "스케줄러 시작 — 등록된 작업 {}개. 종료는 Ctrl-C.",
        store.tasks.len()
    ));
    if !cfg.briefing_time.trim().is_empty() {
        ui::info(&format!(
            "매일 {} 자동 브리핑이 켜져 있어요(설정된 채널로 푸시).",
            cfg.briefing_time
        ));
    }
    // 무인 실행이지만 위험 명령은 기본 차단(allow_dangerous=false).
    let ctx = ToolContext {
        auto_approve: true,
        allow_dangerous: false,
    };
    let tick = std::time::Duration::from_secs(30);
    let mut last_briefed: Option<String> = None;

    loop {
        // 매 틱마다 저장소를 다시 읽어 추가/삭제를 반영한다.
        let mut store = cron::CronStore::load()?;
        let now = cron::now_ms();
        let due_ids: Vec<u64> = store
            .tasks
            .iter()
            .filter(|t| cron::is_due(t, now))
            .map(|t| t.id)
            .collect();

        for id in due_ids {
            let prompt = match store.tasks.iter().find(|t| t.id == id) {
                Some(t) => t.prompt.clone(),
                None => continue,
            };
            ui::note(&format!("▶ 예약 작업 #{id} 실행: {prompt}"));

            let mem = memory::Memory::load()?;
            let skills = skill::SkillStore::load()?;
            let mut messages = vec![
                Message::system(agent::system_prompt(
                    mem.prompt_block(),
                    skills.prompt_block(),
                )),
                Message::user(prompt),
            ];
            match eng.run(cfg, &ctx, &mut messages).await {
                Ok(answer) => agent::print_answer(&answer),
                Err(e) => ui::error(&format!("작업 #{id} 오류: {e:#}")),
            }

            // 실행 시각 기록.
            if let Some(t) = store.tasks.iter_mut().find(|t| t.id == id) {
                t.last_run_ms = Some(cron::now_ms());
            }
            store.save().ok();
        }

        // 약속·알림 확인: 때가 된 알림을 데스크탑 알림 + 푸시 채널로 띄운다.
        check_due_reminders(cfg);

        // 매일 자동 브리핑(설정된 시각이 지났고 오늘 아직 안 보냈으면).
        maybe_send_briefing(eng, cfg, &ctx, &mut last_briefed).await;

        // 코인 시세 알림: 목표가에 도달한 알림을 푸시한다.
        check_price_watches(cfg).await;

        tokio::time::sleep(tick).await;
    }
}

/// 목표가에 도달한 시세 알림을 푸시하고 발동 표시한다(코인 + 환율).
async fn check_price_watches(cfg: &Config) {
    let mut store = match watch::WatchStore::load() {
        Ok(s) => s,
        Err(_) => return,
    };
    let active: Vec<watch::Watch> = store.active().into_iter().cloned().collect();
    if active.is_empty() {
        return;
    }

    // 코인 시세(업비트).
    let coin_markets: Vec<String> = {
        let mut m: Vec<String> = active
            .iter()
            .filter(|w| w.kind != "fx")
            .map(|w| w.market.clone())
            .collect();
        m.sort();
        m.dedup();
        m
    };
    let coins = if coin_markets.is_empty() {
        Vec::new()
    } else {
        coin::fetch(&coin_markets).await.unwrap_or_default()
    };

    // 환율(open.er-api).
    let has_fx = active.iter().any(|w| w.kind == "fx");
    let rates = if has_fx {
        exchange::fetch().await.ok().map(|(_, r)| r)
    } else {
        None
    };

    let mut changed = false;
    for w in &active {
        let price = if w.kind == "fx" {
            rates.as_ref().and_then(|r| exchange::krw_per(&w.symbol, r))
        } else {
            coins.iter().find(|c| c.symbol == w.symbol).map(|c| c.price)
        };
        if let Some(p) = price {
            if watch::should_trigger(w, p) {
                let dir = if w.above { "도달" } else { "하락" };
                ui::note(&format!("🔔 시세 알림: {} {dir}!", w.symbol));
                push::push_blocking(
                    cfg,
                    &format!(
                        "🔔 {} {}원 {dir}! (목표 {}원)",
                        w.symbol,
                        exchange::comma(p, 0),
                        exchange::comma(w.target, 0)
                    ),
                );
                store.mark_triggered(w.id);
                changed = true;
            }
        }
    }
    if changed {
        store.save().ok();
    }
}

/// 설정된 아침 시각이 되면 브리핑을 생성해 푸시 채널로 보낸다.
async fn maybe_send_briefing(
    eng: &Engine,
    cfg: &Config,
    ctx: &ToolContext,
    last_briefed: &mut Option<String>,
) {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    if !briefing::should_brief(
        &cfg.briefing_time,
        last_briefed.as_deref(),
        &today,
        now.hour(),
        now.minute(),
    ) {
        return;
    }
    *last_briefed = Some(today);
    ui::note("☀️ 아침 브리핑을 생성하는 중…");

    let Some(p) = preset::find("브리핑") else {
        return;
    };
    let mem = memory::Memory::load().ok().and_then(|m| m.prompt_block());
    let skills = skill::SkillStore::load()
        .ok()
        .and_then(|s| s.prompt_block());
    let mut messages = vec![
        Message::system(agent::system_prompt(mem, skills)),
        Message::user(p.prompt),
    ];
    match eng.run(cfg, ctx, &mut messages).await {
        Ok(Some(answer)) => {
            let sent = push::push(cfg, &answer).await;
            ui::note(&format!("아침 브리핑 전송({sent}개 채널)."));
        }
        Ok(None) => {}
        Err(e) => ui::error(&format!("브리핑 생성 오류: {e:#}")),
    }
}

/// 때가 된 약속·알림을 띄우고 처리 표시한다(데스크탑 + 푸시 채널).
fn check_due_reminders(cfg: &Config) {
    let mut store = match reminders::ReminderStore::load() {
        Ok(s) => s,
        Err(_) => return,
    };
    let now = reminders::now_unix();
    let due = store.due(now);
    if due.is_empty() {
        return;
    }
    for r in &due {
        ui::note(&format!("🔔 알림: {}", r.title));
        reminders::desktop_notify("원장 알림 🔔", &r.title);
        // 설정된 채널(디스코드/텔레그램)로도 푸시 → 외출 중에도 받음.
        push::push_blocking(cfg, &format!("🔔 {}", r.title));
        // 반복이면 다음 회차로 재예약, 아니면 완료 표시.
        store.handle_fired(r.id, now);
    }
    store.save().ok();
}

fn cmd_notify(cfg: &Config, message: &[String]) -> Result<()> {
    let msg = message.join(" ");
    if msg.trim().is_empty() {
        ui::error("보낼 메시지가 필요합니다. 예: wonjang notify \"집에 가는 중\"");
        std::process::exit(1);
    }
    let channels = push::configured_channels(cfg);
    if channels.is_empty() {
        ui::error("설정된 푸시 채널이 없습니다.");
        ui::info(
            "디스코드: WONJANG_DISCORD_WEBHOOK 에 웹훅 URL을 설정하거나,\n  \
             텔레그램: 토큰 + telegram_allowed_ids 를 설정하세요.",
        );
        std::process::exit(1);
    }
    let sent = push::push_blocking(cfg, &msg);
    if sent == 0 {
        ui::error(&format!(
            "푸시 실패 — 채널 설정(토큰/웹훅)을 확인하세요. (설정된 채널: {})",
            channels.join(", ")
        ));
    } else {
        ui::note(&format!(
            "{sent}개 채널로 푸시했습니다 ({})",
            channels.join(", ")
        ));
    }
    Ok(())
}

/// 시간대별 인사(아침/낮/저녁/밤).
fn greeting() -> &'static str {
    use chrono::Timelike;
    match chrono::Local::now().hour() {
        5..=10 => "좋은 아침이에요 ☀️",
        11..=16 => "좋은 오후예요 🌤️",
        17..=20 => "좋은 저녁이에요 🌆",
        _ => "편안한 밤 되세요 🌙",
    }
}

/// 모든 데이터를 타임스탬프 폴더로 백업한다 — LLM 없이 즉시.
fn cmd_backup(dest: &Option<String>) -> Result<()> {
    let dest_dir = match dest {
        Some(d) => std::path::PathBuf::from(d),
        None => {
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("홈 디렉터리를 찾을 수 없습니다"))?
        }
    };
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let (path, count) = backup::backup(&dest_dir, &ts)?;
    ui::note(&format!("✅ {count}개 파일을 백업했습니다."));
    ui::info(&format!("   위치: {}", path.display()));
    Ok(())
}

fn cmd_restore(source: &str) -> Result<()> {
    let src = std::path::PathBuf::from(source);
    if !src.exists() {
        ui::error(&format!("백업 폴더를 찾을 수 없습니다: {source}"));
        std::process::exit(1);
    }
    let data = backup::data_dir()?;
    // 안전: 복원 전 현재 데이터를 자동 백업(되돌릴 수 있게).
    if data.exists() {
        if let Some(home) = dirs::home_dir() {
            let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
            if let Ok((path, n)) = backup::backup(&home, &format!("{ts}-prerestore")) {
                ui::info(&format!(
                    "복원 전 현재 데이터를 백업했어요({n}개): {}",
                    path.display()
                ));
            }
        }
    }
    let n = backup::restore(&src, &data)?;
    ui::note(&format!("✅ {n}개 파일을 복원했습니다."));
    ui::info("기존 데이터는 위 'prerestore' 백업으로 되돌릴 수 있어요.");
    Ok(())
}

/// 원장의 기능을 카테고리별로 안내한다.
fn cmd_guide() -> Result<()> {
    use owo_colors::OwoColorize;
    let groups: &[(&str, &[(&str, &str)])] = &[
        (
            "💬 대화 & 작업",
            &[
                ("wonjang", "대화형 모드(자연어로 무엇이든)"),
                ("wonjang \"...\"", "한 줄 요청을 바로 실행"),
                ("wonjang preset list", "자주 쓰는 작업 프리셋 보기"),
            ],
        ),
        (
            "🌐 실시간 정보 (키 불필요)",
            &[
                ("wonjang 날씨 [지역]", "실시간 날씨"),
                ("wonjang 미세먼지 [지역]", "PM10·PM2.5 + 등급"),
                ("wonjang 지하철 <역>", "서울 지하철 실시간 도착"),
                ("wonjang 혼잡도 <지역>", "서울 실시간 혼잡도"),
                ("wonjang 따릉이 <대여소>", "서울 따릉이 실시간"),
                ("wonjang 환율 [금액 통화]", "실시간 환율·환산"),
                ("wonjang 코인 [심볼]", "업비트 코인 시세"),
                ("wonjang 뉴스 [검색어]", "최신 뉴스 헤드라인"),
                ("wonjang 공휴일 [년도]", "한국 공휴일(설날·추석 포함)"),
                ("wonjang 긱뉴스 [개수]", "개발·기술·스타트업 뉴스"),
                ("wonjang 깃헙 <owner/repo>", "GitHub 저장소 정보"),
            ],
        ),
        (
            "📅 일정 & 집중",
            &[
                (
                    "wonjang remind add <분> \"약속\"",
                    "약속·알림(반복 --every)",
                ),
                ("wonjang 할일 / todo", "할 일 체크리스트"),
                ("wonjang dday add \"수능\" <날짜>", "디데이"),
                ("wonjang 집중 <분> [무엇]", "뽀모도로 타이머"),
            ],
        ),
        (
            "📒 기록 & 지식",
            &[
                ("wonjang 지출 add <금액> <분류>", "가계부"),
                ("wonjang 습관 done <이름>", "습관 트래커(연속일수)"),
                ("wonjang 일지/메모 (프리셋)", "옵시디언 노트"),
                ("wonjang notion search \"...\"", "노션 검색/기록"),
            ],
        ),
        (
            "📲 알림 & 편의",
            &[
                ("wonjang notify \"메시지\"", "카카오/디스코드/텔레그램 푸시"),
                ("wonjang 열기 <이름>", "즐겨찾기/URL 열기"),
                ("wonjang 로또", "로또 자동번호"),
            ],
        ),
        (
            "🧮 생활·금융 계산기 (키 불필요)",
            &[
                ("wonjang 실수령 <연봉만원>", "연봉 실수령액(4대보험+세금)"),
                ("wonjang 시급 <시급> <주시간>", "주급·월급+주휴수당"),
                ("wonjang 대출 <원금> <%> <개월>", "대출 상환(원리금/원금)"),
                ("wonjang 예금/적금 <...>", "예적금 만기(세후)"),
                ("wonjang 할인 <원가> <%>...", "할인가(중복 할인)"),
                ("wonjang 부가세 <금액>", "공급가/세액 분리"),
                ("wonjang 나이 <YYYY-MM-DD>", "만 나이·연 나이"),
                ("wonjang 날짜 <날짜> [날짜2]", "두 날짜 사이 일수"),
                ("wonjang 평 <숫자>", "평↔㎡ 변환"),
                ("wonjang 변환 <값> <단위>", "온도/무게/길이"),
                ("wonjang bmi <키> <몸무게>", "BMI(아시아 기준)"),
                ("wonjang 더치 <총액> <인원>", "더치페이(n빵)"),
                ("wonjang 뽑기 <후보들>", "제비뽑기/추첨"),
                ("wonjang 메뉴 [카테고리]", "오늘 뭐 먹지?"),
                ("wonjang 글자수 \"<텍스트>\"", "자소서·SNS 글자수"),
                ("wonjang 초성 \"<텍스트>\"", "한글 초성 추출"),
                ("wonjang 영타 \"<한글>\"", "한글→영문 타자(dkssud)"),
                ("wonjang 한타 <영문>", "영문→한글 복원(잘못 친 글자)"),
                ("wonjang 금액 <숫자>", "한글 금액(계약서·수표)"),
                ("wonjang 계산 \"<식>\"", "사칙연산 계산기"),
                ("wonjang 시간 09:00 + 8:30", "시간 더하기/빼기"),
                ("wonjang 진법 255", "2/8/10/16진수 변환"),
            ],
        ),
        (
            "🤖 24시간 자동화",
            &[
                ("wonjang cron add \"@daily\" \"...\"", "예약 작업"),
                ("wonjang cron run", "스케줄러 켜기(알림·자동브리핑)"),
                ("wonjang telegram", "텔레그램으로 원격 조작"),
            ],
        ),
        (
            "📂 내 파일 다루기",
            &[
                ("wonjang 엑셀 <파일.csv>", "엑셀·CSV 요약·미리보기"),
                (
                    "wonjang 엑셀 <파일> --열 금액",
                    "특정 열 합계·평균·최대·최소",
                ),
                ("wonjang 또간집 <지역>", "풍자 또간집 선정 맛집(지역)"),
                ("wonjang 용량 [폴더]", "큰 파일·폴더 찾기(용량 분석)"),
                ("wonjang 중복 [폴더]", "내용 같은 중복 파일 찾기"),
                ("wonjang 정리 <폴더>", "종류별 자동 분류(미리보기→--실행)"),
                ("wonjang 이름변경 <폴더> A B", "파일명 A를 B로 일괄 치환"),
                ("wonjang 압축 <폴더>", "zip 압축 / 압축풀기 <zip>"),
                ("wonjang 찾기 <폴더> <단어>", "파일 내용 검색(grep)"),
                ("wonjang json <파일>", "JSON 검증·정렬·값추출(--키)"),
                ("wonjang 해시 <파일>", "SHA-256 체크섬(무결성 --확인)"),
                ("wonjang 비교 <파일1> <파일2>", "두 파일 줄 단위 diff"),
                ("wonjang qr <URL>", "QR 코드 생성(와이파이 --wifi)"),
            ],
        ),
        (
            "📊 한눈에",
            &[
                ("wonjang 현황", "약속·할일·디데이·습관·집중·지출"),
                ("wonjang preset run 브리핑", "아침 브리핑(날씨·뉴스·일정)"),
                ("wonjang config", "설정·연동 상태"),
            ],
        ),
    ];

    println!();
    println!(
        "  {}  {}",
        "원장 — 한국어 우선 24시간 비서".bright_cyan().bold(),
        "할 수 있는 일".dimmed()
    );
    for (title, items) in groups {
        println!("\n  {}", title.bright_white().bold());
        for (cmd, desc) in *items {
            println!("     {:<34} {}", cmd.bright_green(), desc.dimmed());
        }
    }
    println!();
    ui::info("백엔드: API 키가 있으면 그걸로, 없으면 Claude Code/Codex를 자동 연결합니다.");
    println!();
    Ok(())
}

/// 비서 현황 대시보드(약속·할일·디데이·예약작업) — LLM 없이 즉시.
/// `~`/`~/...`를 홈 경로로 펼친다.
fn expand_path(root: &str) -> std::path::PathBuf {
    if let Some(rest) = root.strip_prefix("~/") {
        dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| std::path::PathBuf::from(root))
    } else if root == "~" {
        dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
    } else {
        std::path::PathBuf::from(root)
    }
}

fn cmd_diff(a: &str, b: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let pa = expand_path(a);
    let pb = expand_path(b);
    let result = diff::diff_files(&pa.to_string_lossy(), &pb.to_string_lossy())?;
    println!();
    println!("  📑 비교: {} ↔ {}", a.dimmed(), b.dimmed());
    if result.added == 0 && result.removed == 0 {
        println!("     두 파일이 같아요 👍");
        println!();
        return Ok(());
    }
    println!(
        "     {} 추가 · {} 삭제",
        format!("+{}", result.added).green(),
        format!("-{}", result.removed).red()
    );
    println!();
    let mut shown = 0;
    for line in &result.lines {
        // 변경 없는 줄이 너무 많으면 생략(맥락 약간만).
        match line.tag {
            '+' => println!("  {}", format!("+ {}", line.text).green()),
            '-' => println!("  {}", format!("- {}", line.text).red()),
            _ => {
                if shown < 400 {
                    println!("    {}", line.text.dimmed());
                }
            }
        }
        shown += 1;
        if shown > 1000 {
            println!("  … (너무 길어 일부만 표시)");
            break;
        }
    }
    println!();
    Ok(())
}

fn cmd_hash(file: &str, algo: &str, verify: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    let path = expand_path(file);
    if !path.exists() {
        return Err(anyhow::anyhow!("파일이 없어요: {}", path.display()));
    }
    let algo = hash::Algo::parse(algo)?;
    let digest = hash::file_digest(&path, algo)?;
    println!();
    println!("  🔐 {} 체크섬", algo.name());
    println!("     {}", digest.bright_cyan());
    if let Some(expected) = verify {
        let expected = expected.trim().to_lowercase();
        if expected == digest {
            println!("     ✅ 일치 — 파일이 변조되지 않았어요");
        } else {
            println!("     ❌ 불일치! 파일이 다르거나 손상됐을 수 있어요");
            println!("        기대값: {}", expected.dimmed());
        }
    }
    println!();
    Ok(())
}

fn cmd_json(file: &str, key: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    let path = expand_path(file);
    let value = jsontool::parse_file(&path.to_string_lossy())?;
    println!();
    if let Some(k) = key {
        match jsontool::pick(&value, k) {
            Some(v) => {
                println!("  🔑 {} = {}", k.bright_cyan(), jsontool::summary(v));
                println!();
                println!("{}", jsontool::pretty(v));
            }
            None => println!("  '{k}' 경로를 찾지 못했어요."),
        }
        println!();
        return Ok(());
    }
    println!(
        "  ✅ 올바른 JSON — {} ({})",
        path.display().to_string().bright_cyan(),
        jsontool::summary(&value)
    );
    // 너무 크면 통째로 쏟지 않는다.
    let pretty = jsontool::pretty(&value);
    if pretty.len() <= 4000 {
        println!();
        println!("{pretty}");
    } else {
        println!(
            "     ({}바이트 — 일부만; 값 추출은 --키 사용)",
            pretty.len()
        );
        if let serde_json::Value::Object(m) = &value {
            println!(
                "     최상위 키: {}",
                m.keys().cloned().collect::<Vec<_>>().join(", ")
            );
        }
    }
    println!();
    Ok(())
}

fn cmd_search(path: &str, query: &str, max: usize) -> Result<()> {
    use owo_colors::OwoColorize;
    let dir = expand_path(path);
    if !dir.exists() {
        return Err(anyhow::anyhow!("경로가 없어요: {}", dir.display()));
    }
    let max = max.clamp(1, 1000);
    let result = search::search(&dir, query, max);
    println!();
    println!(
        "  🔎 '{}' 검색 — {}건 (파일 {}개 훑음)",
        query.bright_cyan(),
        result.matches.len(),
        result.files_scanned
    );
    if result.matches.is_empty() {
        println!("     찾지 못했어요.");
        println!();
        return Ok(());
    }
    // 파일별로 묶어 보여준다.
    let mut last_file: Option<&std::path::Path> = None;
    for m in &result.matches {
        if last_file != Some(m.file.as_path()) {
            println!();
            println!("  {}", m.file.display().to_string().bold());
            last_file = Some(m.file.as_path());
        }
        // 검색어를 강조.
        println!(
            "     {}: {}",
            m.line_no.to_string().dimmed(),
            highlight(&m.line, query)
        );
    }
    if result.truncated {
        println!();
        println!("  …상한({max})에 도달했어요. --개수 로 더 보세요.");
    }
    println!();
    Ok(())
}

/// 줄에서 검색어(대소문자 무시)를 굵게 표시.
fn highlight(line: &str, query: &str) -> String {
    use owo_colors::OwoColorize;
    let lower = line.to_lowercase();
    let q = query.to_lowercase();
    let Some(pos) = lower.find(&q) else {
        return line.to_string();
    };
    // 바이트 위치를 문자 경계로 맞춰 안전하게 자른다.
    let end = pos + q.len();
    if !line.is_char_boundary(pos) || !line.is_char_boundary(end) {
        return line.to_string();
    }
    format!(
        "{}{}{}",
        &line[..pos],
        (&line[pos..end]).bright_yellow().bold(),
        &line[end..]
    )
}

fn cmd_zip(sources: &[String], output: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    if sources.is_empty() {
        println!();
        println!("  압축할 폴더/파일을 알려주세요. 예: wonjang 압축 ~/문서");
        println!();
        return Ok(());
    }
    let srcs: Vec<std::path::PathBuf> = sources.iter().map(|s| expand_path(s)).collect();
    // 출력 이름: 지정 없으면 첫 소스 이름 기반.
    let out = match output {
        Some(o) => expand_path(o),
        None => {
            let stem = srcs[0]
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "archive".to_string());
            srcs[0]
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(format!("{stem}.zip"))
        }
    };
    println!();
    println!(
        "  🗜️  압축 중… → {}",
        out.display().to_string().bright_cyan()
    );
    let n = archive::create_zip(&srcs, &out)?;
    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    println!(
        "  ✅ 완료: {n}개 파일 → {} ({})",
        out.display(),
        diskusage::human(size)
    );
    println!();
    Ok(())
}

fn cmd_unzip(file: &str, dest: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    let zip_path = expand_path(file);
    if !zip_path.exists() {
        return Err(anyhow::anyhow!("zip 파일이 없어요: {}", zip_path.display()));
    }
    let target = match dest {
        Some(d) => expand_path(d),
        None => {
            // zip 이름(확장자 제거)의 새 폴더로.
            let stem = zip_path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unzipped".to_string());
            zip_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(stem)
        }
    };
    println!();
    println!(
        "  🗜️  푸는 중… → {}",
        target.display().to_string().bright_cyan()
    );
    let n = archive::extract_zip(&zip_path, &target)?;
    println!("  ✅ 완료: {n}개 파일 → {}", target.display());
    println!();
    Ok(())
}

fn cmd_rename(path: &str, find: &str, replace: &str, run: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    let dir = expand_path(path);
    let plans = rename::plan(&dir, find, replace)?;
    println!();
    if plans.is_empty() {
        println!("  ✏️  '{find}'가 들어간 파일이 없어요: {}", dir.display());
        println!();
        return Ok(());
    }
    if run {
        let n = rename::execute(&dir, &plans)?;
        println!("  ✏️  이름 변경 완료: {n}개 ('{find}' → '{replace}')");
    } else {
        println!(
            "  ✏️  이름 변경 미리보기: {}개 ('{}' → '{}')",
            plans.len(),
            find.bright_cyan(),
            replace.bright_cyan()
        );
        for r in plans.iter().take(20) {
            let old = r.from.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            println!("     {}  →  {}", old.dimmed(), r.to_name.bold());
        }
        if plans.len() > 20 {
            println!("     …외 {}개", plans.len() - 20);
        }
        println!();
        println!(
            "  {} 실제로 바꾸려면: {}",
            "▶".green(),
            format!("wonjang 이름변경 {path} {find} {replace} --실행").bold()
        );
    }
    println!();
    Ok(())
}

fn cmd_organize(path: &str, run: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::collections::BTreeMap;
    let dir = expand_path(path);
    let plans = organize::plan(&dir)?;
    println!();
    if plans.is_empty() {
        println!(
            "  🗂️  정리할 파일이 없어요(최상위 파일 기준): {}",
            dir.display()
        );
        println!();
        return Ok(());
    }
    // 카테고리별 개수 집계.
    let mut by_cat: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for m in &plans {
        let name = m
            .from
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        by_cat.entry(m.category).or_default().push(name);
    }

    if run {
        let moved = organize::execute(&dir, &plans)?;
        println!("  🗂️  정리 완료: {} → {moved}개 파일 이동", dir.display());
        for (cat, files) in &by_cat {
            println!("     {}/  {}개", cat.bold(), files.len());
        }
    } else {
        println!(
            "  🗂️  정리 미리보기: {} ({}개 파일)",
            dir.display().to_string().bright_cyan(),
            plans.len()
        );
        for (cat, files) in &by_cat {
            println!("     {}/  {}개", cat.bold(), files.len());
            for f in files.iter().take(4) {
                println!("       - {}", f.dimmed());
            }
            if files.len() > 4 {
                println!("       …외 {}개", files.len() - 4);
            }
        }
        println!();
        println!(
            "  {} 실제로 옮기려면: {}",
            "▶".green(),
            format!("wonjang 정리 {path} --실행").bold()
        );
    }
    println!();
    Ok(())
}

fn cmd_dedup(path: Option<&str>, top: usize) -> Result<()> {
    use owo_colors::OwoColorize;
    let expanded = expand_path(path.unwrap_or("."));
    if !expanded.exists() {
        return Err(anyhow::anyhow!("경로가 없어요: {}", expanded.display()));
    }
    let top = top.clamp(1, 50);
    println!();
    println!(
        "  🔁 중복 파일 검사: {} (해시 비교 중…)",
        expanded.display().to_string().bright_cyan()
    );
    let result = dedup::find_duplicates(&expanded);
    if result.groups.is_empty() {
        println!("     중복 파일이 없어요 👍");
        println!();
        return Ok(());
    }
    println!(
        "     중복 묶음 {}개 · 낭비 용량 {}",
        result.groups.len(),
        diskusage::human(result.total_wasted).bold()
    );
    println!();
    for (i, g) in result.groups.iter().take(top).enumerate() {
        println!(
            "  {}. {} × {}벌  (낭비 {})",
            i + 1,
            diskusage::human(g.size),
            g.paths.len(),
            diskusage::human(g.wasted())
        );
        for p in &g.paths {
            println!("     - {}", p.display().to_string().dimmed());
        }
    }
    if result.groups.len() > top {
        println!();
        println!(
            "  …외 {}개 묶음 더 (--개수 로 더 보기)",
            result.groups.len() - top
        );
    }
    println!();
    println!(
        "  {} 읽기 전용입니다. 지울 파일은 직접 확인 후 삭제하세요.",
        "ⓘ".dimmed()
    );
    println!();
    Ok(())
}

fn cmd_disk(path: Option<&str>, top: usize) -> Result<()> {
    use owo_colors::OwoColorize;
    let expanded = expand_path(path.unwrap_or("."));

    let top = top.clamp(1, 50);
    println!();
    println!(
        "  💾 용량 분석: {} (훑는 중…)",
        expanded.display().to_string().bright_cyan()
    );
    let usage = diskusage::analyze(&expanded, top)
        .map_err(|e| anyhow::anyhow!("폴더를 읽을 수 없어요: {e}"))?;

    println!(
        "     총 {} · 파일 {}개",
        diskusage::human(usage.total).bold(),
        usage.file_count
    );
    if !usage.largest_dirs.is_empty() {
        println!();
        println!("  📁 큰 폴더 Top {}", usage.largest_dirs.len());
        for (p, size) in &usage.largest_dirs {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            println!("     {:>9}  {}/", diskusage::human(*size), name);
        }
    }
    if !usage.largest_files.is_empty() {
        println!();
        println!("  📄 큰 파일 Top {}", usage.largest_files.len());
        for (p, size) in &usage.largest_files {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            println!("     {:>9}  {}", diskusage::human(*size), name);
        }
    }
    println!();
    Ok(())
}

fn cmd_ddoganjip(
    query: Option<&str>,
    add: Option<&str>,
    region: Option<&str>,
    note: &str,
) -> Result<()> {
    use owo_colors::OwoColorize;
    let mut store = ddoganjip::DdoganjipStore::load()?;

    // 추가 모드.
    if let Some(name) = add {
        let region = region.ok_or_else(|| {
            anyhow::anyhow!(
                "지역도 알려주세요. 예: wonjang 또간집 --추가 \"○○식당\" --지역 \"서울 종로\""
            )
        })?;
        store.add(name, region, note)?;
        println!();
        println!("  ✅ 또간집 목록에 추가: {name} ({region})");
        println!();
        return Ok(());
    }

    let hits = match query {
        Some(q) => store.find(q),
        None => store.items.iter().collect(),
    };

    println!();
    match query {
        Some(q) => println!("  🍜 또간집 — '{q}' 검색 ({}곳)", hits.len()),
        None => println!("  🍜 또간집 선정 맛집 (전체 {}곳)", hits.len()),
    }
    if hits.is_empty() {
        println!("     결과가 없어요. `wonjang 또간집`으로 전체를 보거나,");
        println!("     `wonjang 또간집 --추가 \"식당\" --지역 \"지역\"`으로 추가하세요.");
        println!();
        return Ok(());
    }
    for s in &hits {
        let tag = if s.verified {
            "✔ 확인".green().to_string()
        } else {
            "※ 미검증".yellow().to_string()
        };
        println!("     • {}  ({})  [{}]", s.name.bold(), s.region, tag);
        if !s.note.is_empty() {
            println!("       {}", s.note.dimmed());
        }
    }
    println!();
    println!(
        "  {} 〈또간집〉 공식 목록 API가 없어 직접 키우는 목록입니다.",
        "ⓘ".dimmed()
    );
    println!(
        "    확인한 곳 추가: {}",
        "wonjang 또간집 --추가 \"식당\" --지역 \"서울 종로\"".dimmed()
    );
    println!();
    Ok(())
}

fn cmd_excel(file: &str, column: Option<&str>, preview_rows: usize, json: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    let table = sheet::Table::load(file)?;

    // JSON 변환 모드: 표를 [{헤더: 값}] 배열로 출력.
    if json {
        let arr: Vec<serde_json::Value> = table
            .rows
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                for (i, header) in table.headers.iter().enumerate() {
                    let cell = row.get(i).cloned().unwrap_or_default();
                    obj.insert(header.clone(), serde_json::Value::String(cell));
                }
                serde_json::Value::Object(obj)
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::Value::Array(arr))?
        );
        return Ok(());
    }

    println!();
    println!(
        "  📊 {} — {}행 × {}열",
        file.bright_cyan(),
        table.rows.len(),
        table.headers.len()
    );

    // 특정 열 통계.
    if let Some(key) = column {
        let idx = table.col_index(key).ok_or_else(|| {
            anyhow::anyhow!(
                "'{key}' 열을 찾을 수 없어요. 열: {}",
                table.headers.join(", ")
            )
        })?;
        let nums = table.numeric_column(idx);
        println!();
        println!("  📈 '{}' 열 통계", table.headers[idx]);
        if nums.is_empty() {
            println!("     숫자 값이 없어요(텍스트 열).");
        } else {
            let sum: f64 = nums.iter().sum();
            let avg = sum / nums.len() as f64;
            let max = nums.iter().cloned().fold(f64::MIN, f64::max);
            let min = nums.iter().cloned().fold(f64::MAX, f64::min);
            let fmt = |v: f64| {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    expenses::won(v as i64).trim_end_matches('원').to_string()
                } else {
                    format!("{v:.2}")
                }
            };
            println!("     개수   {}개 (숫자 {}개)", table.rows.len(), nums.len());
            println!("     합계   {}", fmt(sum));
            println!("     평균   {}", fmt(avg));
            println!("     최대   {}", fmt(max));
            println!("     최소   {}", fmt(min));
        }
        println!();
        return Ok(());
    }

    // 열 목록 + 미리보기.
    println!("  열: {}", table.headers.join(" · ").dimmed());
    let n = preview_rows.min(table.rows.len());
    if n > 0 {
        println!();
        println!("  미리보기 (상위 {n}행):");
        for row in table.rows.iter().take(n) {
            let cells: Vec<String> = row
                .iter()
                .map(|c| {
                    let c = c.trim();
                    if c.chars().count() > 16 {
                        format!("{}…", c.chars().take(15).collect::<String>())
                    } else {
                        c.to_string()
                    }
                })
                .collect();
            println!("     {}", cells.join(" | "));
        }
    }
    println!();
    println!(
        "  {} 특정 열 합계·평균: {}",
        "팁".dimmed(),
        format!("wonjang 엑셀 {file} --열 <열이름>").dimmed()
    );
    println!();
    Ok(())
}

fn cmd_status() -> Result<()> {
    use owo_colors::OwoColorize;
    let now_unix = reminders::now_unix();
    let today = ddays::today();

    println!();
    println!(
        "  {}  {}",
        "원장 현황".bright_cyan().bold(),
        greeting().dimmed()
    );
    println!();

    // 다가오는 약속(최대 3).
    let rem = reminders::ReminderStore::load()?;
    let upcoming = rem.upcoming(now_unix);
    println!("  ⏰ 약속");
    if upcoming.is_empty() {
        ui::info("     예정된 약속이 없어요.");
    } else {
        for r in upcoming.iter().take(3) {
            println!(
                "     · {} ({}{})",
                r.title,
                reminders::relative(r.at_unix, now_unix),
                reminders::repeat_label(r.repeat_secs)
            );
        }
    }

    // 할 일(최대 5).
    let todo = todos::TodoStore::load()?;
    let pending = todo.pending();
    println!("  ✅ 할 일 ({}개)", pending.len());
    for t in pending.iter().take(5) {
        println!("     ☐ {}", t.text);
    }
    if pending.len() > 5 {
        ui::info(&format!("     … 외 {}개", pending.len() - 5));
    }

    // 디데이(가까운 3).
    let dd = ddays::DdayStore::load()?;
    if !dd.all().is_empty() {
        println!("  📅 디데이");
        for d in dd.all().iter().take(3) {
            let label = ddays::parse_date(&d.date)
                .map(|dt| ddays::dday_label(ddays::days_until(dt, today)))
                .unwrap_or_else(|_| "?".to_string());
            println!("     {} {}", label.bright_yellow(), d.label);
        }
    }

    // 습관(오늘 완료/전체).
    let habit = habits::HabitStore::load()?;
    if !habit.items.is_empty() {
        let today_s = habits::today_str();
        let done = habit
            .items
            .iter()
            .filter(|h| h.done_today(&today_s))
            .count();
        println!("  🔥 습관: 오늘 {}/{} 완료", done, habit.items.len());
        let pending: Vec<&str> = habit
            .items
            .iter()
            .filter(|h| !h.done_today(&today_s))
            .map(|h| h.name.as_str())
            .take(4)
            .collect();
        if !pending.is_empty() {
            ui::info(&format!("     남은 습관: {}", pending.join(", ")));
        }
    }

    // 오늘 집중 시간.
    let foc = focus::FocusStore::load()?;
    let foc_today = focus::today_str();
    let foc_min = foc.today_total(&foc_today);
    if foc_min > 0 {
        println!("  🍅 오늘 집중: {}", focus::fmt_minutes(foc_min));
    }

    // 오늘 지출.
    let exp = expenses::ExpenseStore::load()?;
    let exp_today = exp.total_on(&expenses::today_str());
    if exp_today > 0 {
        println!("  💰 오늘 지출: {}", expenses::won(exp_today));
    }

    // 예약 작업.
    let cron = cron::CronStore::load()?;
    let enabled = cron.tasks.iter().filter(|t| t.enabled).count();
    if enabled > 0 {
        println!("  🔁 예약 작업 {enabled}개 등록됨");
    }

    println!();
    Ok(())
}

fn cmd_bookmark(action: &Option<BookmarkAction>) -> Result<()> {
    let mut store = bookmarks::BookmarkStore::load()?;
    match action {
        Some(BookmarkAction::Add { name, target }) => {
            let id = store.add(name, target)?;
            ui::note(&format!("즐겨찾기 #{id} 추가: {name} → {target}"));
        }
        Some(BookmarkAction::Remove { id }) => {
            if store.remove(*id)? {
                ui::note(&format!("즐겨찾기 #{id}을(를) 삭제했습니다."));
            } else {
                ui::error(&format!("즐겨찾기 #{id}을(를) 찾을 수 없습니다."));
            }
        }
        None | Some(BookmarkAction::List) => {
            if store.items.is_empty() {
                ui::info("즐겨찾기가 없어요. 추가: wonjang 즐겨찾기 add 노션 https://notion.so");
                return Ok(());
            }
            println!("즐겨찾기:\n");
            for b in &store.items {
                println!("  #{}  {}  →  {}", b.id, b.name, b.target);
            }
            println!();
            ui::info("열기: wonjang 열기 <이름>");
        }
    }
    Ok(())
}

fn cmd_watch(action: &Option<WatchAction>) -> Result<()> {
    let mut store = watch::WatchStore::load()?;
    match action {
        Some(WatchAction::Add { symbol, target }) => {
            let sym = symbol.to_uppercase();
            // 통화(USD/JPY 등)면 환율 감시, 아니면 코인 감시.
            let is_fx = !exchange::currency_name(&sym).is_empty();
            let kind = if is_fx { "fx" } else { "coin" };
            // 현재가로 알림 방향 결정(현재가보다 높으면 '이상', 낮으면 '이하').
            let current = util::run_async({
                let sym = sym.clone();
                async move {
                    if is_fx {
                        let (_, rates) = exchange::fetch().await?;
                        Ok(exchange::krw_per(&sym, &rates))
                    } else {
                        let coins = coin::fetch(&[format!("KRW-{sym}")]).await?;
                        Ok(coins.first().map(|c| c.price))
                    }
                }
            })?;
            let above = match current {
                Some(c) => *target >= c,
                None => true,
            };
            let id = store.add(&sym, *target, above, kind)?;
            let dir = if above { "이상" } else { "이하" };
            let unit = if is_fx { "원(1단위)" } else { "원" };
            ui::note(&format!(
                "시세 알림 #{id}: {sym}이(가) {}{unit} {dir}이면 푸시",
                exchange::comma(*target, 0)
            ));
            if let Some(c) = current {
                ui::info(&format!("현재 {}원", exchange::comma(c, 0)));
            }
            ui::info("감시하려면 스케줄러를 켜 두세요: wonjang cron run");
        }
        Some(WatchAction::Remove { id }) => {
            if store.remove(*id)? {
                ui::note(&format!("시세 알림 #{id}을(를) 삭제했습니다."));
            } else {
                ui::error(&format!("시세 알림 #{id}을(를) 찾을 수 없습니다."));
            }
        }
        None | Some(WatchAction::List) => {
            if store.items.is_empty() {
                ui::info("등록된 시세 알림이 없어요. 예: wonjang 감시 add BTC 110000000");
                return Ok(());
            }
            println!("시세 알림:\n");
            for w in &store.items {
                let dir = if w.above { "≥" } else { "≤" };
                let state = if w.triggered { " (발동됨)" } else { "" };
                println!(
                    "  #{}  {} {dir} {}원{state}",
                    w.id,
                    w.symbol,
                    exchange::comma(w.target, 0)
                );
            }
            println!();
        }
    }
    Ok(())
}

fn cmd_pyeong(value: f64) -> Result<()> {
    println!();
    println!("  📐 평수 변환");
    println!("     {value:.0}평 = {:.1}㎡", pyeong::pyeong_to_m2(value));
    println!("     {value:.0}㎡ = {:.1}평", pyeong::m2_to_pyeong(value));
    println!();
    Ok(())
}

fn cmd_age(birth: &str) -> Result<()> {
    let birth = age::parse_birth(birth)?;
    let today = chrono::Local::now().date_naive();
    let man = age::korean_age(birth, today);
    let yeon = age::year_age(birth, today);
    let dday = age::days_to_birthday(birth, today);
    println!();
    let animal = age::zodiac_animal(chrono::Datelike::year(&birth));
    let sign = age::star_sign(
        chrono::Datelike::month(&birth),
        chrono::Datelike::day(&birth),
    );
    println!("  🎂 나이 계산 ({})", birth.format("%Y년 %m월 %d일생"));
    println!("     만 나이: {man}세");
    println!("     연 나이: {yeon}세  (현재 연도 − 출생 연도)");
    println!("     띠: {animal}띠   별자리: {sign}");
    if dday == 0 {
        println!("     🎉 오늘이 생일이에요!");
    } else {
        println!("     다음 생일까지 {dday}일 남았어요");
    }
    println!();
    Ok(())
}

fn cmd_salary(manwon: f64) -> Result<()> {
    let annual = manwon * 10_000.0;
    let p = salary::from_annual(annual);
    let w = |v: f64| expenses::won(v.round() as i64);
    println!();
    println!("  💰 연봉 실수령액 ({}만 원)", manwon as i64);
    println!("     월 세전        {}", w(p.gross_monthly));
    println!("     ─ 국민연금     -{}", w(p.national_pension));
    println!("     ─ 건강보험     -{}", w(p.health));
    println!("     ─ 장기요양     -{}", w(p.long_term_care));
    println!("     ─ 고용보험     -{}", w(p.employment));
    println!("     ─ 소득세       -{}", w(p.income_tax));
    println!("     ─ 지방소득세   -{}", w(p.local_tax));
    println!("     ───────────────");
    println!("     월 실수령      {}", w(p.net_monthly()));
    println!("     연 실수령      {}", w(p.net_monthly() * 12.0));
    println!();
    println!("  ※ 2025년 요율·1인 가구·비과세 식대 20만 원 기준 추정치");
    println!();
    Ok(())
}

fn cmd_loan(manwon: f64, rate: f64, months: u32) -> Result<()> {
    let principal = manwon * 10_000.0;
    let ep = loan::equal_payment(principal, rate, months);
    let pp = loan::equal_principal(principal, rate, months);
    let w = |v: f64| expenses::won(v.round() as i64);
    let years = months / 12;
    let rest = months % 12;
    let term = if rest == 0 {
        format!("{years}년")
    } else if years == 0 {
        format!("{rest}개월")
    } else {
        format!("{years}년 {rest}개월")
    };
    println!();
    println!(
        "  🏦 대출 상환 ({}만 원 · 연 {rate}% · {term})",
        manwon as i64
    );
    println!();
    println!("  [원리금균등] 매달 같은 금액");
    println!("     월 상환액      {}", w(ep.monthly));
    println!("     총 이자        {}", w(ep.total_interest));
    println!("     총 상환액      {}", w(ep.total_payment));
    println!();
    println!("  [원금균등] 매달 원금 동일, 이자 감소");
    println!("     첫 달          {}", w(pp.first_month));
    println!("     마지막 달      {}", w(pp.last_month));
    println!("     총 이자        {}", w(pp.total_interest));
    println!("     총 상환액      {}", w(pp.total_payment));
    println!();
    Ok(())
}

fn cmd_deposit(manwon: f64, rate: f64, months: u32, is_savings: bool) -> Result<()> {
    let amount = manwon * 10_000.0;
    let m = if is_savings {
        deposit::installment(amount, rate, months)
    } else {
        deposit::time_deposit(amount, rate, months)
    };
    let w = |v: f64| expenses::won(v.round() as i64);
    println!();
    if is_savings {
        println!(
            "  🐷 정기적금 ({}만 원/월 · 연 {rate}% · {months}개월)",
            manwon as i64
        );
        println!("     원금 합계      {}", w(m.principal));
    } else {
        println!(
            "  🏦 정기예금 ({}만 원 · 연 {rate}% · {months}개월)",
            manwon as i64
        );
        println!("     예치 원금      {}", w(m.principal));
    }
    println!("     세전 이자      {}", w(m.interest_pretax));
    println!("     ─ 이자소득세   -{}  (15.4%)", w(m.tax));
    println!("     세후 이자      {}", w(m.interest_aftertax));
    println!("     ───────────────");
    println!("     만기 수령액    {}", w(m.total));
    println!();
    Ok(())
}

fn cmd_bike(cfg: &Config, query: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    let key = cfg.seoul_api_key.clone();
    let q = query.unwrap_or("").to_string();
    let (stations, is_sample) = util::run_async(async move { bike::fetch(&key, &q).await })?;
    println!();
    match query {
        Some(q) => println!("  🚲 따릉이 — '{q}' ({}곳)", stations.len()),
        None => println!("  🚲 따릉이 대여소 ({}곳)", stations.len()),
    }
    if stations.is_empty() {
        println!("     해당 대여소를 못 찾았어요. 이름 일부로 다시 검색해 보세요.");
    }
    for s in &stations {
        let mark = if s.bikes == 0 {
            "🈳".to_string()
        } else {
            format!("🚲 {}", s.bikes)
        };
        println!("     {}  {} / 거치대 {}", mark, s.name.bold(), s.racks);
    }
    if is_sample {
        println!();
        println!(
            "  {} 'sample' 키라 고정 예시(망원역 일대)만 나와요. 서울 무료 키를 넣으면 전체 조회.",
            "ⓘ".dimmed()
        );
    }
    println!();
    Ok(())
}

fn cmd_github(slug: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let (owner, repo) = github::split_slug(slug)?;
    let owner = owner.to_string();
    let repo = repo.to_string();
    let (info, release) = util::run_async(async move {
        let info = github::fetch_repo(&owner, &repo).await?;
        let release = github::fetch_latest_release(&owner, &repo).await;
        Ok::<_, anyhow::Error>((info, release))
    })?;
    println!();
    println!("  🐙 {}", info.full_name.bold());
    if let Some(desc) = &info.description {
        if !desc.is_empty() {
            println!("     {}", desc.dimmed());
        }
    }
    let lang = info.language.as_deref().unwrap_or("?");
    println!(
        "     ★ {} · 🍴 {} · 이슈 {} · {}",
        info.stargazers_count, info.forks_count, info.open_issues_count, lang
    );
    if let Some(pushed) = &info.pushed_at {
        // ISO 8601에서 날짜 부분만.
        let date = pushed.split('T').next().unwrap_or(pushed);
        println!("     최근 푸시: {date}");
    }
    match release {
        Some(r) => println!(
            "     최신 릴리스: {} {}",
            r.tag_name.bright_cyan(),
            r.name.unwrap_or_default().dimmed()
        ),
        None => println!("     {}", "(릴리스 없음)".dimmed()),
    }
    println!();
    Ok(())
}

fn cmd_qr(text: &[String], wifi: Option<&str>, password: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let (data, label) = match wifi {
        Some(ssid) => (
            qr::wifi_payload(ssid, password),
            format!("와이파이: {ssid}"),
        ),
        None => {
            if text.is_empty() {
                println!();
                println!("  QR로 만들 내용을 입력하세요. 예: wonjang qr https://example.com");
                println!("  와이파이: wonjang qr --wifi <SSID> --비번 <비밀번호>");
                println!();
                return Ok(());
            }
            let joined = text.join(" ");
            let label = joined.clone();
            (joined, label)
        }
    };
    let rendered = qr::render_terminal(&data)?;
    println!();
    println!("  📱 {}", label.bright_cyan());
    println!();
    println!("{rendered}");
    println!("  휴대폰 카메라로 스캔하세요.");
    println!();
    Ok(())
}

fn cmd_geeknews(count: Option<usize>) -> Result<()> {
    use owo_colors::OwoColorize;
    let n = count.unwrap_or(10).clamp(1, 30);
    let items = util::run_async(async move { geeknews::fetch(n).await })?;
    println!();
    println!("  🤓 긱뉴스 — 개발·기술·스타트업 ({}건)", items.len());
    if items.is_empty() {
        println!("     불러오지 못했어요. 잠시 후 다시 시도하세요.");
        println!();
        return Ok(());
    }
    for (i, it) in items.iter().enumerate() {
        println!("  {:>2}. {}", i + 1, it.title.bold());
        if !it.link.is_empty() {
            println!("      {}", it.link.dimmed());
        }
    }
    println!();
    Ok(())
}

fn cmd_congestion(cfg: &Config, area: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let key = cfg.seoul_api_key.clone();
    let q = area.to_string();
    let c = util::run_async(async move { congestion::fetch(&key, &q).await })?;
    println!();
    println!(
        "  {} {} — {}",
        congestion::level_emoji(&c.level),
        c.area.bold(),
        c.level.bright_cyan()
    );
    if !c.message.is_empty() {
        println!("     {}", c.message);
    }
    if !c.ppltn_min.is_empty() {
        println!("     실시간 인구: 약 {}~{}명", c.ppltn_min, c.ppltn_max);
    }
    if !c.time.is_empty() {
        println!("     기준: {}", c.time.dimmed());
    }
    if c.is_sample {
        println!();
        println!(
            "  {} 지금은 'sample' 키라 고정 예시(광화문)만 나와요.",
            "ⓘ".dimmed()
        );
        println!(
            "    data.seoul.go.kr 무료 키를 발급해 {} 에 넣으면 원하는 지역이 나옵니다.",
            "config(seoul_api_key)".dimmed()
        );
    }
    println!();
    Ok(())
}

fn cmd_holiday(year: Option<i32>) -> Result<()> {
    use owo_colors::OwoColorize;
    let today = chrono::Local::now().date_naive();
    let year = year.unwrap_or_else(|| chrono::Datelike::year(&today));
    let holidays = util::run_async(async move { holidays::fetch(year).await })?;
    println!();
    println!("  🗓️  {year}년 한국 공휴일 ({}일)", holidays.len());
    if holidays.is_empty() {
        println!("     데이터를 가져오지 못했어요.");
        println!();
        return Ok(());
    }

    // 같은 이름의 연속된 날짜(연휴)를 한 줄로 묶는다.
    let mut i = 0;
    while i < holidays.len() {
        let start = &holidays[i];
        let mut end = start;
        let mut j = i + 1;
        while j < holidays.len()
            && holidays[j].name == start.name
            && (holidays[j].date - holidays[j - 1].date).num_days() == 1
        {
            end = &holidays[j];
            j += 1;
        }
        let date_str = if start.date == end.date {
            format!(
                "{} ({})",
                start.date.format("%m-%d"),
                datecalc::weekday_kr(start.date)
            )
        } else {
            format!(
                "{}~{}",
                start.date.format("%m-%d"),
                end.date.format("%m-%d")
            )
        };
        // D-day(미래면 표시).
        let dday = datecalc::days_between(today, start.date);
        let dlabel = if start.date <= today && today <= end.date {
            "오늘!".bright_red().to_string()
        } else if dday > 0 {
            format!("D-{dday}").dimmed().to_string()
        } else {
            "지남".dimmed().to_string()
        };
        println!("     {:<14} {}  {}", date_str, start.name.bold(), dlabel);
        i = j;
    }

    if let Some(next) = holidays::next_after(&holidays, today) {
        let dday = datecalc::days_between(today, next.date);
        println!();
        if dday == 0 {
            println!("  🎉 오늘은 {}!", next.name);
        } else {
            println!("  👉 다음 공휴일: {} 까지 D-{}", next.name, dday);
        }
    }
    println!();
    Ok(())
}

fn cmd_date(from: Option<&str>, to: Option<&str>, plus: Option<i64>) -> Result<()> {
    let today = chrono::Local::now().date_naive();
    let base = match from {
        Some(s) => datecalc::parse(s)?,
        None => today,
    };
    let fmt =
        |d: chrono::NaiveDate| format!("{} ({})", d.format("%Y-%m-%d"), datecalc::weekday_kr(d));
    println!();
    if let Some(to) = to {
        // 두 날짜 사이 일수.
        let target = datecalc::parse(to)?;
        let days = datecalc::days_between(base, target);
        let weeks = days.abs() / 7;
        let rest = days.abs() % 7;
        println!("  📅 날짜 사이");
        println!("     {}  →  {}", fmt(base), fmt(target));
        println!("     {days}일 ({weeks}주 {rest}일)");
    } else if let Some(n) = plus {
        // N일 후/전 날짜.
        let result = datecalc::add_days(base, n);
        let word = if n >= 0 { "후" } else { "전" };
        println!("  📅 {} 기준 {}일 {word}", fmt(base), n.abs());
        println!("     👉 {}", fmt(result));
    } else {
        // 인자가 today 하나뿐이면 오늘(또는 그 날짜) 정보.
        let day_of_year = chrono::Datelike::ordinal(&base);
        println!("  📅 {}", fmt(base));
        println!("     올해 {day_of_year}번째 날");
        if from.is_none() {
            println!("     (오늘)");
        }
    }
    println!();
    Ok(())
}

fn cmd_radix(value: &str) -> Result<()> {
    println!();
    match radix::parse(value) {
        Ok(n) => {
            let r = radix::all(n);
            println!("  🔢 진법 변환 ({value})");
            println!("     10진수  {}", r.decimal);
            println!("     2진수   {}", r.binary);
            println!("     8진수   {}", r.octal);
            println!("     16진수  {}", r.hex);
        }
        Err(e) => println!("  ⚠️ {e}"),
    }
    println!();
    Ok(())
}

fn cmd_time(items: &[String]) -> Result<()> {
    println!();
    if items.is_empty() {
        println!("  계산할 시간을 입력하세요. 예: wonjang 시간 09:00 + 8:30");
        println!();
        return Ok(());
    }
    match timecalc::sum(items) {
        Ok(total) => {
            println!("  ⏱️  {}", items.join(" "));
            println!("     = {}  ({}분)", timecalc::format_hm(total), total);
            println!("     시계: {}", timecalc::format_clock(total));
        }
        Err(e) => println!("  ⚠️ {e}"),
    }
    println!();
    Ok(())
}

fn cmd_calc(expr: &[String]) -> Result<()> {
    println!();
    if expr.is_empty() {
        println!("  계산할 식을 입력하세요. 예: wonjang 계산 \"15000 * 1.1\"");
        println!();
        return Ok(());
    }
    let joined = expr.join(" ");
    match calc::eval(&joined) {
        Ok(v) => {
            // 정수면 콤마 구분, 아니면 소수 6자리까지(불필요한 0 제거).
            let pretty = if v.fract() == 0.0 && v.abs() < 1e15 {
                expenses::won(v as i64).trim_end_matches('원').to_string()
            } else {
                let s = format!("{v:.6}");
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            };
            println!("  🧮 {joined}");
            println!("     = {pretty}");
        }
        Err(e) => println!("  ⚠️ {e}"),
    }
    println!();
    Ok(())
}

fn cmd_amount(value: u64) -> Result<()> {
    println!();
    println!("  💴 한글 금액");
    println!("     {}", expenses::won(value as i64));
    println!("     👉 일금 {}원정", koreannum::to_korean(value));
    println!();
    Ok(())
}

fn cmd_keystroke(text: &[String]) -> Result<()> {
    println!();
    if text.is_empty() {
        println!("  변환할 한글을 입력하세요. 예: wonjang 영타 \"안녕\"");
        println!();
        return Ok(());
    }
    let joined = text.join(" ");
    println!("  ⌨️  한글 → 영문 타자");
    println!("     {joined}");
    println!("     👉 {}", keyboard::han_to_eng(&joined));
    println!();
    Ok(())
}

fn cmd_hanstroke(text: &[String]) -> Result<()> {
    println!();
    if text.is_empty() {
        println!("  복원할 영문을 입력하세요. 예: wonjang 한타 dkssud");
        println!();
        return Ok(());
    }
    let joined = text.join(" ");
    println!("  ⌨️  영문 → 한글 복원");
    println!("     {joined}");
    println!("     👉 {}", keyboard::eng_to_han(&joined));
    println!();
    Ok(())
}

fn cmd_choseong(text: &[String]) -> Result<()> {
    println!();
    if text.is_empty() {
        println!("  초성을 뽑을 텍스트를 입력하세요. 예: wonjang 초성 \"안녕하세요\"");
        println!();
        return Ok(());
    }
    let joined = text.join(" ");
    println!("  🔡 초성 추출");
    println!("     {joined}");
    println!("     👉 {}", hangul::choseong(&joined));
    println!();
    Ok(())
}

fn cmd_chars(text: &[String]) -> Result<()> {
    println!();
    if text.is_empty() {
        println!("  셀 텍스트를 입력하세요. 예: wonjang 글자수 \"자기소개서 내용\"");
        println!();
        return Ok(());
    }
    let joined = text.join(" ");
    let c = charcount::count(&joined);
    println!("  ✍️  글자수 세기");
    println!("     공백 포함      {}자", c.chars_with_space);
    println!("     공백 제외      {}자", c.chars_without_space);
    println!("     단어 수        {}개", c.words);
    println!("     줄 수          {}줄", c.lines);
    println!("     바이트         {}B", c.bytes);
    println!();
    Ok(())
}

fn cmd_wage(hourly: f64, weekly_hours: f64) -> Result<()> {
    let w = |v: f64| expenses::won(v.round() as i64);
    let r = wage::calc(hourly, weekly_hours);
    println!();
    println!(
        "  💵 시급 계산 (시급 {} · 주 {:.0}시간)",
        w(r.hourly),
        r.weekly_hours
    );
    println!("     주 기본급      {}", w(r.base_weekly));
    if r.holiday_pay > 0.0 {
        println!("     주휴수당       {}  (주 15시간↑)", w(r.holiday_pay));
    } else {
        println!("     주휴수당       없음  (주 15시간 미만)");
    }
    println!("     주급 합계      {}", w(r.weekly_total));
    println!("     월 환산        {}  (주급×4.345)", w(r.monthly));
    if r.below_min {
        println!(
            "     ⚠️ 2025년 최저시급({}) 미만입니다",
            w(wage::MIN_WAGE_2025)
        );
    }
    println!();
    Ok(())
}

fn cmd_vat(amount: f64) -> Result<()> {
    let w = |v: f64| expenses::won(v.round() as i64);
    let (s1, v1, t1) = vat::from_supply(amount);
    let (s2, v2, _t2) = vat::from_total(amount);
    println!();
    println!("  🧾 부가세 계산 ({}, VAT 10%)", w(amount));
    println!();
    println!("  ▸ 이 금액이 공급가액이면");
    println!("     세액           {}", w(v1));
    println!("     합계(VAT 포함) {}", w(t1));
    println!("     (공급가 {})", w(s1));
    println!();
    println!("  ▸ 이 금액이 VAT 포함 합계이면");
    println!("     공급가액       {}", w(s2));
    println!("     세액           {}", w(v2));
    println!();
    Ok(())
}

fn cmd_discount(price: f64, rates: &[f64]) -> Result<()> {
    let w = |v: f64| expenses::won(v.round() as i64);
    println!();
    if rates.is_empty() {
        println!("  할인율을 입력하세요. 예: wonjang 할인 30000 20 (또는 20 10 중복)");
        println!();
        return Ok(());
    }
    let d = discount::apply(price, rates);
    let rate_str = rates
        .iter()
        .map(|r| format!("{r:.0}%"))
        .collect::<Vec<_>>()
        .join(" + ");
    println!("  🏷️  할인가 계산 ({} · {rate_str})", w(d.original));
    println!("     할인가         {}", w(d.final_price));
    println!("     절약액         {}", w(d.saved));
    if rates.len() > 1 {
        println!("     실질 할인율    {:.1}%", d.effective_rate);
    }
    println!();
    Ok(())
}

fn cmd_bmi(height: f64, weight: f64) -> Result<()> {
    println!();
    match bmi::calc(height, weight) {
        Some(b) => {
            println!("  ⚖️  BMI 계산 ({height:.0}cm · {weight:.0}kg)");
            println!("     BMI            {:.1}", b.value);
            println!("     판정           {}  (아시아 기준)", b.grade);
            println!("     표준체중       {:.1}kg  (BMI 22)", b.standard_kg);
            let diff = weight - b.standard_kg;
            if diff.abs() >= 0.5 {
                let word = if diff > 0.0 { "초과" } else { "부족" };
                println!("     표준 대비      {:.1}kg {word}", diff.abs());
            }
        }
        None => println!("  키와 몸무게는 0보다 커야 해요."),
    }
    println!();
    Ok(())
}

fn cmd_convert(value: f64, unit: &str) -> Result<()> {
    println!();
    match convert::convert(value, unit) {
        Some(c) => println!("  📏 {}", c.label),
        None => println!("  '{unit}' 단위는 몰라요. 가능: {}", convert::supported()),
    }
    println!();
    Ok(())
}

fn cmd_pick(items: &[String], count: usize, order: bool) -> Result<()> {
    println!();
    if items.len() < 2 {
        println!("  뽑을 후보를 2개 이상 입력하세요. 예: wonjang 뽑기 철수 영희 민수");
        println!();
        return Ok(());
    }
    if order {
        // draw가 시간 시드로 전체를 섞어 순서를 만든다.
        let full = pick::draw(items, items.len());
        println!("  🎲 순서 정하기");
        for (i, it) in full.iter().enumerate() {
            println!("     {}. {}", i + 1, it);
        }
    } else {
        let n = count.clamp(1, items.len());
        let winners = pick::draw(items, n);
        if n == 1 {
            println!("  🎯 당첨!  👉 {}", winners[0]);
        } else {
            println!("  🎯 {n}명 당첨!");
            for w in &winners {
                println!("     • {w}");
            }
        }
    }
    println!();
    Ok(())
}

fn cmd_dutch(total: i64, people: i64, unit: i64) -> Result<()> {
    let w = expenses::won;
    println!();
    match dutchpay::split(total, people, unit) {
        Some(s) => {
            println!("  🧾 더치페이 ({} · {}명)", w(s.total), s.people);
            println!(
                "     1인당          {}  ({}원 단위 올림)",
                w(s.per_person),
                unit
            );
            println!("     정확히 나누면  {:.1}원", s.exact);
            println!("     걷히는 총액    {}", w(s.collected));
            if s.leftover > 0 {
                println!("     남는 거스름    {}  (총무 보관)", w(s.leftover));
            } else {
                println!("     딱 떨어져요 👍");
            }
        }
        None => println!("  총액은 0 이상, 인원은 1명 이상이어야 해요."),
    }
    println!();
    Ok(())
}

fn cmd_menu(category: Option<&str>) -> Result<()> {
    if let Some(name) = category {
        if menu::find_category(name).is_none() {
            println!();
            println!(
                "  '{name}' 카테고리는 없어요. 가능: {}",
                menu::category_keys().join(" / ")
            );
            println!();
            return Ok(());
        }
    }
    println!();
    match menu::recommend(category) {
        Some((cat, m)) => {
            println!("  🍽️  오늘 뭐 먹지?");
            println!("     👉 [{cat}] {m}");
        }
        None => println!("  추천할 메뉴를 찾지 못했어요."),
    }
    println!();
    Ok(())
}

fn cmd_lotto(games: Option<usize>) -> Result<()> {
    let n = games.unwrap_or(5).clamp(1, 10);
    println!();
    println!("  🎱 로또 자동 번호 ({n}게임)");
    for (i, g) in lotto::auto(n).iter().enumerate() {
        let label = (b'A' + i as u8) as char;
        let nums: Vec<String> = g.iter().map(|x| format!("{x:2}")).collect();
        println!("     {label}  {}", nums.join("  "));
    }
    println!();
    ui::info("재미로 즐기세요 🍀");
    Ok(())
}

fn cmd_news(query: &[String]) -> Result<()> {
    let q = query.join(" ");
    let qo = if q.trim().is_empty() {
        None
    } else {
        Some(q.clone())
    };
    let list = util::run_async(async move { news::headlines(qo.as_deref(), 8).await })?;
    if list.is_empty() {
        ui::info("뉴스를 가져오지 못했어요.");
        return Ok(());
    }
    println!();
    if q.trim().is_empty() {
        println!("  📰 주요 뉴스");
    } else {
        println!("  📰 '{q}' 뉴스");
    }
    for h in &list {
        println!("     · {h}");
    }
    println!();
    Ok(())
}

fn cmd_coin(symbol: &Option<String>) -> Result<()> {
    use owo_colors::OwoColorize;
    let markets = match symbol {
        Some(s) if !s.trim().is_empty() => vec![format!("KRW-{}", s.trim().to_uppercase())],
        _ => coin::default_markets(),
    };
    let coins = util::run_async(async move { coin::fetch(&markets).await })?;
    if coins.is_empty() {
        ui::info("시세를 찾지 못했어요. 심볼을 확인해 주세요(예: BTC).");
        return Ok(());
    }
    println!();
    println!("  🪙 코인 시세 (업비트)");
    for c in &coins {
        let name = coin::coin_name(&c.symbol);
        let pct = format!("{:+.2}%", c.change_pct);
        let colored = if c.change_pct >= 0.0 {
            pct.red().to_string() // 한국 관습: 상승=빨강
        } else {
            pct.blue().to_string() // 하락=파랑
        };
        println!(
            "     {} {:<10} {}원  {}",
            c.symbol,
            name,
            exchange::comma(c.price, 0),
            colored
        );
    }
    println!();
    Ok(())
}

fn cmd_exchange(amount: Option<f64>, currency: &Option<String>) -> Result<()> {
    let (date, rates) = util::run_async(async move { exchange::fetch().await })?;
    println!();
    match currency {
        Some(cur) => {
            let per = exchange::krw_per(cur, &rates)
                .ok_or_else(|| anyhow::anyhow!("'{cur}' 환율을 찾을 수 없습니다"))?;
            let amt = amount.unwrap_or(1.0);
            println!(
                "  💱 {} {} = {}원",
                exchange::comma(amt, 0),
                cur.to_uppercase(),
                exchange::comma(amt * per, 0)
            );
        }
        None => {
            println!("  💱 환율 (원화 기준)");
            for (code, unit) in [("USD", 1.0), ("JPY", 100.0), ("EUR", 1.0), ("CNY", 1.0)] {
                if let Some(per) = exchange::krw_per(code, &rates) {
                    let name = exchange::currency_name(code);
                    println!(
                        "     {} {code}({name}) = {}원",
                        exchange::comma(unit, 0),
                        exchange::comma(unit * per, 0)
                    );
                }
            }
        }
    }
    ui::info(&format!("     ({})", date.trim()));
    println!();
    Ok(())
}

fn cmd_air(location: &[String]) -> Result<()> {
    let loc = location.join(" ");
    let a = util::run_async(async move { airquality::air_quality(&loc).await })?;
    let (g25, e25) = airquality::grade_pm25(a.pm25);
    let (g10, e10) = airquality::grade_pm10(a.pm10);
    println!();
    println!("  🌫 {} 미세먼지", a.place);
    println!("     미세먼지(PM10)    {:.0}  {} {}", a.pm10, g10, e10);
    println!("     초미세먼지(PM2.5) {:.0}  {} {}", a.pm25, g25, e25);
    println!();
    Ok(())
}

fn cmd_weather(location: &[String]) -> Result<()> {
    let loc = location.join(" ");
    let w = util::run_async(async move { weather::weather(&loc).await })?;
    println!();
    println!(
        "  ☀️ {} 날씨: {} {:.0}°C (체감 {:.0}°C)",
        w.place, w.desc, w.temp, w.feels
    );
    println!(
        "     습도 {}% · 강수 {}mm · 오늘 {:.0}~{:.0}°C",
        w.humidity, w.precip, w.today_min, w.today_max
    );
    println!();
    Ok(())
}

fn cmd_subway(cfg: &Config, station: &str) -> Result<()> {
    let key = cfg.seoul_api_key.clone();
    let st = station.to_string();
    let list = util::run_async(async move { subway::arrivals(&key, &st, 10).await })?;
    if list.is_empty() {
        ui::info(&format!(
            "'{station}' 도착 정보가 없어요. 역 이름을 확인하거나 잠시 후 다시 시도하세요."
        ));
        return Ok(());
    }
    println!("\n  🚇 {station} 실시간 도착:\n");
    for a in &list {
        println!("  [{}] {} — {}", a.line, a.direction, a.message);
    }
    println!();
    Ok(())
}

fn cmd_open(target: &str) -> Result<()> {
    let store = bookmarks::BookmarkStore::load()?;
    // 즐겨찾기 이름이면 그 대상을, 아니면 입력 자체(URL/경로)를 연다.
    let (label, to_open) = match store.find(target) {
        Some(b) => (b.name.clone(), b.target.clone()),
        None => (target.to_string(), target.to_string()),
    };
    bookmarks::open_target(&to_open)?;
    ui::note(&format!("'{label}' 열었어요 → {to_open}"));
    Ok(())
}

fn cmd_focus(minutes: Option<i64>, label: &[String]) -> Result<()> {
    let today = focus::today_str();
    match minutes {
        Some(m) if m > 0 => {
            let label = label.join(" ");
            // 세션 기록.
            let mut store = focus::FocusStore::load()?;
            store.add(m, &label)?;
            // 끝나는 시각에 알림 등록(스케줄러가 켜져 있으면 울림).
            let title = if label.is_empty() {
                "집중 완료! 🎉".to_string()
            } else {
                format!("집중 완료: {label} 🎉")
            };
            let mut rem = reminders::ReminderStore::load()?;
            rem.add(reminders::now_unix() + m * 60, &title, None)?;

            let what = if label.is_empty() {
                String::new()
            } else {
                format!(" ({label})")
            };
            ui::note(&format!("⏳ 집중 시작{what} — {}분", m));
            ui::info(&format!(
                "{}분 뒤 알림이 울려요(스케줄러: wonjang cron run). 오늘 누적 {}",
                m,
                focus::fmt_minutes(store.today_total(&today))
            ));
        }
        Some(_) => {
            ui::error("집중 시간은 1분 이상이어야 합니다. 예: wonjang 집중 25 코딩");
        }
        None => {
            let store = focus::FocusStore::load()?;
            let total = store.today_total(&today);
            let count = store.today_count(&today);
            if count == 0 {
                ui::info("오늘 집중 기록이 없어요. 시작: wonjang 집중 25 코딩");
            } else {
                println!();
                println!(
                    "  🍅 오늘 집중: {} ({}회 세션)",
                    focus::fmt_minutes(total),
                    count
                );
                println!();
            }
        }
    }
    Ok(())
}

fn cmd_habit(action: &Option<HabitAction>) -> Result<()> {
    let mut store = habits::HabitStore::load()?;
    match action {
        Some(HabitAction::Add { name }) => {
            let id = store.add(name)?;
            ui::note(&format!("습관 #{id} 추가: {name}. 오늘부터 시작해 봐요!"));
        }
        Some(HabitAction::Done { habit }) => match store.check(habit)? {
            Some((name, streak)) => ui::note(&format!("'{name}' 완료! 🔥 {streak}일 연속")),
            None => ui::error(&format!("'{habit}' 습관을 찾을 수 없습니다.")),
        },
        Some(HabitAction::Remove { id }) => {
            if store.remove(*id)? {
                ui::note(&format!("습관 #{id}을(를) 삭제했습니다."));
            } else {
                ui::error(&format!("습관 #{id}을(를) 찾을 수 없습니다."));
            }
        }
        None | Some(HabitAction::List) => {
            if store.items.is_empty() {
                ui::info("등록된 습관이 없어요. 추가: wonjang 습관 add \"운동\"");
                return Ok(());
            }
            let today = habits::today();
            let today_s = habits::today_str();
            println!("습관:\n");
            for h in &store.items {
                let mark = if h.done_today(&today_s) { "✓" } else { "·" };
                println!("  {} #{}  {}  🔥{}일", mark, h.id, h.name, h.streak(today));
            }
            println!();
            ui::info("완료: wonjang 습관 done <이름>   |   추가: wonjang 습관 add \"<이름>\"");
        }
    }
    Ok(())
}

fn cmd_expense(action: &Option<ExpenseAction>) -> Result<()> {
    let mut store = expenses::ExpenseStore::load()?;
    let today = expenses::today_str();
    let ym = expenses::this_month();
    match action {
        Some(ExpenseAction::Add {
            amount,
            category,
            note,
        }) => {
            let note = note.join(" ");
            let id = store.add(*amount, category, &note)?;
            ui::note(&format!(
                "지출 #{id} 기록: {} ({category})",
                expenses::won(*amount)
            ));
            ui::info(&format!(
                "오늘 합계 {} · 이번 달 {}",
                expenses::won(store.total_on(&today)),
                expenses::won(store.total_in_month(&ym))
            ));
        }
        Some(ExpenseAction::Remove { id }) => {
            if store.remove(*id)? {
                ui::note(&format!("지출 #{id}을(를) 삭제했습니다."));
            } else {
                ui::error(&format!("지출 #{id}을(를) 찾을 수 없습니다."));
            }
        }
        Some(ExpenseAction::Month) => {
            let by = store.by_category_in_month(&ym);
            if by.is_empty() {
                ui::info("이번 달 지출 기록이 없어요.");
                return Ok(());
            }
            println!("이번 달({ym}) 분류별 지출:\n");
            for (cat, amt) in by {
                println!("  {cat:<8} {}", expenses::won(amt));
            }
            println!("\n  합계: {}", expenses::won(store.total_in_month(&ym)));
        }
        None => {
            println!();
            println!(
                "  💰 오늘({today}) 지출: {}",
                expenses::won(store.total_on(&today))
            );
            println!(
                "     이번 달({ym}) 지출: {}",
                expenses::won(store.total_in_month(&ym))
            );
            let recent = store.recent(5);
            if !recent.is_empty() {
                println!("\n  최근 지출:");
                for e in recent {
                    let note = if e.note.is_empty() {
                        String::new()
                    } else {
                        format!(" - {}", e.note)
                    };
                    println!(
                        "     {} {} ({}){}",
                        e.date,
                        expenses::won(e.amount),
                        e.category,
                        note
                    );
                }
            }
            println!();
            ui::info(
                "기록: wonjang 지출 add <금액> <분류> [메모]   |   이번달: wonjang 지출 month",
            );
        }
    }
    Ok(())
}

fn cmd_notion(cfg: &Config, action: &NotionAction) -> Result<()> {
    let token = cfg.notion_token.trim();
    if token.is_empty() {
        ui::error("노션 토큰이 없습니다.");
        ui::info(
            "환경 변수 WONJANG_NOTION_TOKEN 에 통합 토큰을 설정하고, 대상 페이지/DB의 \
             연결(Connections)에 그 통합을 추가하세요. (notion.so/my-integrations)",
        );
        std::process::exit(1);
    }
    let token = token.to_string();
    match action {
        NotionAction::Search { query } => {
            let q = query.clone();
            let hits = util::run_async(async move { notion::search(&token, &q, 10).await })?;
            if hits.is_empty() {
                ui::info("검색 결과가 없습니다(통합이 해당 페이지에 연결됐는지 확인하세요).");
            } else {
                for h in &hits {
                    println!("[{}] {}", h.kind, h.title);
                    ui::info(&format!("   id: {}", h.id));
                }
            }
        }
        NotionAction::Append { page_id, text } => {
            let (p, t) = (page_id.clone(), text.clone());
            util::run_async(async move { notion::append_paragraph(&token, &p, &t).await })?;
            ui::note("노션 페이지에 기록했습니다.");
        }
    }
    Ok(())
}

fn cmd_dday(action: &Option<DdayAction>) -> Result<()> {
    let mut store = ddays::DdayStore::load()?;
    match action {
        Some(DdayAction::Add { label, date }) => {
            let id = store.add(label, date)?;
            let days = ddays::days_until(ddays::parse_date(date)?, ddays::today());
            ui::note(&format!(
                "디데이 #{id} 등록: {label} ({date}, {})",
                ddays::dday_label(days)
            ));
        }
        Some(DdayAction::Remove { id }) => {
            if store.remove(*id)? {
                ui::note(&format!("디데이 #{id}을(를) 삭제했습니다."));
            } else {
                ui::error(&format!("디데이 #{id}을(를) 찾을 수 없습니다."));
            }
        }
        None | Some(DdayAction::List) => {
            if store.all().is_empty() {
                ui::info("등록된 디데이가 없습니다. 추가: wonjang dday add \"수능\" 2026-11-19");
                return Ok(());
            }
            let today = ddays::today();
            println!("디데이:\n");
            for d in store.all() {
                let label = ddays::parse_date(&d.date)
                    .map(|dt| ddays::dday_label(ddays::days_until(dt, today)))
                    .unwrap_or_else(|_| "?".to_string());
                println!("  {:>7}  {}  ({})", label, d.label, d.date);
            }
            println!();
        }
    }
    Ok(())
}

fn cmd_todo(action: &Option<TodoAction>) -> Result<()> {
    let mut store = todos::TodoStore::load()?;
    match action {
        Some(TodoAction::Add { text }) => {
            if text.trim().is_empty() {
                ui::error("할 일 내용이 필요합니다. 예: wonjang todo add \"장보기\"");
                std::process::exit(1);
            }
            let id = store.add(text)?;
            ui::note(&format!("할 일 #{id} 추가: {text}"));
        }
        Some(TodoAction::Done { id }) => {
            if store.complete(*id)? {
                ui::note(&format!("할 일 #{id} 완료! 👍"));
            } else {
                ui::error(&format!("할 일 #{id}을(를) 찾을 수 없습니다."));
            }
        }
        Some(TodoAction::Remove { id }) => {
            if store.remove(*id)? {
                ui::note(&format!("할 일 #{id}을(를) 삭제했습니다."));
            } else {
                ui::error(&format!("할 일 #{id}을(를) 찾을 수 없습니다."));
            }
        }
        Some(TodoAction::Clear) => {
            let n = store.clear_done()?;
            ui::note(&format!("완료된 할 일 {n}개를 정리했습니다."));
        }
        None | Some(TodoAction::List) => {
            let pending = store.pending();
            if pending.is_empty() {
                ui::info("할 일이 없습니다. 깔끔하네요! 추가: wonjang todo add \"할 일\"");
                return Ok(());
            }
            println!("할 일:\n");
            for t in pending {
                println!("  ☐ #{}  {}", t.id, t.text);
            }
            println!();
            ui::info("완료: wonjang todo done <id>   |   정리: wonjang todo clear");
        }
    }
    Ok(())
}

fn cmd_remind(action: &Option<RemindAction>) -> Result<()> {
    let now = reminders::now_unix();
    if let Some(RemindAction::Add {
        minutes,
        title,
        every,
    }) = action
    {
        if title.trim().is_empty() {
            ui::error("알림 제목이 필요합니다. 예: wonjang remind add 30 \"물 마시기\"");
            std::process::exit(1);
        }
        // 반복 주기 파싱(크론의 스케줄 파서 재사용).
        let repeat = match every {
            Some(e) => Some(cron::parse_schedule(e)?.interval.as_secs() as i64),
            None => None,
        };
        let mut store = reminders::ReminderStore::load()?;
        let at = now + minutes * 60;
        let id = store.add(at, title, repeat)?;
        ui::note(&format!(
            "알림 #{id} 등록: '{title}' ({}{})",
            reminders::relative(at, now),
            reminders::repeat_label(repeat)
        ));
        ui::info("때가 되면 알리려면 스케줄러를 켜 두세요: wonjang cron run");
        return Ok(());
    }
    if let Some(RemindAction::Remove { id }) = action {
        let mut store = reminders::ReminderStore::load()?;
        if store.remove(*id)? {
            ui::note(&format!("알림 #{id}을(를) 삭제했습니다."));
        } else {
            ui::error(&format!("알림 #{id}을(를) 찾을 수 없습니다."));
        }
        return Ok(());
    }

    // 기본: 목록.
    let store = reminders::ReminderStore::load()?;
    let up = store.upcoming(now);
    if up.is_empty() {
        ui::info(
            "예정된 약속·알림이 없습니다. 대화로 등록해 보세요. 예) '내일 오후 3시 치과 알려줘'",
        );
        return Ok(());
    }
    println!("예정된 약속·알림:\n");
    for r in up {
        println!(
            "  #{}  {}  ({}{})",
            r.id,
            r.title,
            reminders::relative(r.at_unix, now),
            reminders::repeat_label(r.repeat_secs)
        );
    }
    println!();
    ui::info("때가 되면 알림을 띄우려면 스케줄러를 켜 두세요: wonjang cron run");
    Ok(())
}

fn cmd_skills() -> Result<()> {
    let store = skill::SkillStore::load()?;
    let skills = store.list()?;
    println!("  스킬 폴더: {}", store.dir().display());
    if skills.is_empty() {
        ui::info("아직 익힌 스킬이 없습니다. 까다로운 작업을 함께 해결하면 쌓입니다.");
        return Ok(());
    }
    println!("\n익힌 스킬 {}개:\n", skills.len());
    for s in &skills {
        println!("  • {}  — {}", s.name, s.description);
    }
    Ok(())
}

fn cmd_memory() -> Result<()> {
    let mem = memory::Memory::load()?;
    println!("  메모리 파일: {}", mem.path().display());
    let content = mem.read();
    if content.trim().is_empty() {
        ui::info("아직 기억하고 있는 사실이 없습니다. 대화하면서 점점 쌓입니다.");
    } else {
        println!("\n{}", content.trim());
    }
    Ok(())
}

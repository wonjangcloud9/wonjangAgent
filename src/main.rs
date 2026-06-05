//! 원장 에이전트 — 로컬 환경을 다루는 한국어 우선 자율 AI 에이전트 (Rust).
//!
//! 헤르메스 에이전트(NousResearch/hermes-agent)의 핵심 아이디어를 러스트로
//! 재구성한다: 제공자 무관 LLM, 로컬 도구, 에이전트 루프, 한국어 우선 UX.

mod age;
mod agent;
mod airquality;
mod annual_leave;
mod archive;
mod backup;
mod bike;
mod bmi;
mod bmr;
mod bookmarks;
mod briefing;
mod calc;
mod car_tax;
mod card;
mod charcount;
mod cli_backend;
mod clipboard;
mod coin;
mod color;
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
mod email;
mod encode;
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
mod jeonse;
mod journal;
mod jsontool;
mod keyboard;
mod koreannum;
mod llm;
mod loan;
mod lotto;
mod mcp;
mod memory;
mod menu;
mod myip;
mod news;
mod notes;
mod notion;
mod organize;
mod password;
mod pick;
mod preset;
mod push;
mod pyeong;
mod qr;
mod radix;
mod reminders;
mod rename;
mod roman;
mod safety;
mod salary;
mod search;
mod session;
mod severance;
mod sheet;
mod skill;
mod sleepcalc;
mod soul;
mod subway;
mod timecalc;
mod timestamp;
mod todos;
mod tools;
mod ui;
mod uptime;
mod util;
mod uuidgen;
mod vat;
mod wage;
mod watch;
mod weather;
mod web;
mod worldtime;

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
    #[command(aliases = ["약속", "알림", "리마인더"])]
    Remind {
        #[command(subcommand)]
        action: Option<RemindAction>,
    },
    /// 할 일(체크리스트)을 보거나 추가/완료합니다.
    #[command(aliases = ["할일", "투두"])]
    Todo {
        #[command(subcommand)]
        action: Option<TodoAction>,
    },
    /// 설정된 채널(디스코드/슬랙/텔레그램/카카오)로 메시지를 푸시합니다.
    Notify {
        /// 보낼 메시지
        #[arg(trailing_var_arg = true)]
        message: Vec<String>,
    },
    /// 디데이(중요한 날까지 남은 일수)를 보거나 등록/삭제합니다.
    #[command(alias = "디데이")]
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
    #[command(aliases = ["압축풀기", "압축해제"])]
    Unzip {
        /// 풀 zip 파일
        file: String,
        /// 풀 폴더(생략 시 zip 이름의 새 폴더)
        dest: Option<String>,
    },
    /// zip을 풀지 않고 안의 목록만 봅니다(한글 파일명 깨짐 보정). 예: wonjang 압축보기 자료.zip
    #[command(name = "압축보기", alias = "압축목록")]
    ZipView {
        /// 들여다볼 zip 파일
        file: String,
    },
    /// 파일 이름 일괄 변경(특정 문자 치환). 예: wonjang 이름변경 ~/사진 IMG_ 여행_
    #[command(aliases = ["이름변경", "이름바꾸기"])]
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
        /// 이 열로 묶어 집계(--열과 함께). 예: --그룹 지점 --열 매출액 → 지점별 매출 합계
        #[arg(long = "그룹")]
        group: Option<String>,
        /// 엑셀 다중 시트에서 분석할 시트(이름 또는 1-기반 번호). 생략 시 첫 시트
        #[arg(long = "시트")]
        sheet: Option<String>,
        /// 조건에 맞는 행만(다른 분석과 조합). 예: --필터 지역=서울 · --필터 매출>1000000 (= != > < >= <= ~포함)
        #[arg(long = "필터")]
        filter: Option<String>,
        /// 이 열로 정렬해 상위 행 보기(기본 큰 값/늦은 가나다부터). 예: --정렬 매출액
        #[arg(long = "정렬")]
        sort: Option<String>,
        /// 정렬을 오름차순(작은 값/가나다부터)으로
        #[arg(long = "오름차순")]
        ascending: bool,
        /// 결과(필터·정렬 행 또는 --그룹 집계)를 새 CSV로 저장. 예: --저장 서울매출.csv
        #[arg(long = "저장", visible_alias = "출력")]
        save: Option<String>,
        /// 미리볼 행 수(기본 5)
        #[arg(long = "행", default_value_t = 5)]
        rows: usize,
        /// 표 전체를 JSON 배열로 출력(헤더를 키로)
        #[arg(long = "json")]
        json: bool,
    },
    /// 여러 표(CSV·엑셀)를 머리글 기준으로 한 파일로 합칩니다. 예: wonjang 표합치기 1월.csv 2월.csv --저장 합본.csv
    #[command(name = "표합치기", aliases = ["엑셀합치기", "csv합치기"])]
    SheetMerge {
        /// 합칠 파일들(같은·비슷한 머리글, 2개 이상). 열 순서가 달라도 이름으로 맞춤
        files: Vec<String>,
        /// 결과 저장 경로(생략 시 미리보기만). 예: --저장 합본.csv
        #[arg(long = "저장", visible_alias = "출력")]
        save: Option<String>,
        /// 맨 앞에 '출처'(각 파일 이름) 열 추가 — 합친 뒤 어느 파일에서 왔는지
        #[arg(long = "출처")]
        source: bool,
    },
    /// 이미지를 줄이거나 압축합니다(여러 장 한 번에, 첨부 용량↓, 원본 보존). 예: wonjang 이미지 *.jpg --폭 1280
    #[command(alias = "이미지")]
    Image {
        /// 이미지 파일들(JPEG/PNG, 여러 장 가능)
        #[arg(required = true)]
        files: Vec<String>,
        /// 최대 가로 폭(px)으로 축소(비율 유지). 원본보다 크면 그대로
        #[arg(long = "폭")]
        width: Option<u32>,
        /// 배율로 축소(0~1, 예: 0.5는 절반)
        #[arg(long = "배율")]
        scale: Option<f64>,
        /// JPEG 압축 품질(1~100, 기본 80)
        #[arg(long = "품질", default_value_t = 80)]
        quality: u8,
        /// 출력 형식 변환(jpg 또는 png). 생략 시 원본과 같은 형식
        #[arg(long = "형식")]
        format: Option<String>,
        /// 저장 경로(생략 시 원본 옆에 _작게/_변환 붙여 저장)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// 여러 사진을 PDF 한 파일로 묶습니다(서류 제출·스캔앱 대용). 예: wonjang 사진묶기 *.jpg
    #[command(alias = "사진묶기")]
    PhotosPdf {
        /// 이미지 파일들(JPEG/PNG, 적은 순서대로 페이지)
        #[arg(required = true)]
        files: Vec<String>,
        /// 저장 경로(생략 시 묶음.pdf)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// 여러 이미지를 한 장으로 이어붙입니다(긴 캡처·영수증 합본). 예: wonjang 이미지이어붙이기 1.png 2.png --세로
    #[command(name = "이미지이어붙이기", aliases = ["사진이어붙이기", "이어붙이기"])]
    ImageStitch {
        /// 이어붙일 이미지들(적은 순서대로, 2개 이상)
        #[arg(required = true)]
        files: Vec<String>,
        /// 세로로 쌓기(기본). 명시적으로 적어도 됩니다
        #[arg(long = "세로")]
        vertical: bool,
        /// 가로로 나란히(기본은 세로로 쌓기)
        #[arg(long = "가로")]
        horizontal: bool,
        /// 저장 경로(생략 시 이어붙임.png)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// 여러 PDF를 하나로 합칩니다(서류 합본 제출). 예: wonjang pdf합치기 a.pdf b.pdf
    #[command(name = "pdf합치기", alias = "피디에프합치기")]
    PdfMerge {
        /// 합칠 PDF들(적은 순서대로 이어 붙임)
        #[arg(required = true)]
        files: Vec<String>,
        /// 저장 경로(생략 시 합본.pdf)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// PDF에 비밀번호를 걸어 보호합니다(민감 서류 제출용). 예: wonjang pdf암호 계약서.pdf --비번 mypw
    #[command(name = "pdf암호", alias = "피디에프암호")]
    PdfEncrypt {
        /// PDF 파일 경로
        file: String,
        /// 걸 비밀번호(이 비번 없이는 못 엽니다)
        #[arg(long = "비번")]
        password: String,
        /// 저장 경로(생략 시 원본 옆에 _암호 붙여 저장)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// 옆으로 스캔된 PDF를 돌립니다(90의 배수). 예: wonjang pdf회전 스캔.pdf 90
    #[command(name = "pdf회전", alias = "피디에프회전")]
    PdfRotate {
        /// PDF 파일 경로
        file: String,
        /// 회전 각도(90의 배수, 기본 90 시계방향). 음수는 반시계
        #[arg(default_value_t = 90)]
        angle: i64,
        /// 특정 페이지만(예: 1-3,5). 생략 시 전체
        #[arg(long = "페이지")]
        pages: Option<String>,
        /// 저장 경로(생략 시 원본 옆에 _회전 붙여 저장)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// PDF에서 원하는 페이지만 새 PDF로 추출합니다. 예: wonjang pdf페이지 보고서.pdf 1-3,5
    #[command(name = "pdf페이지", alias = "피디에프페이지")]
    PdfPages {
        /// PDF 파일 경로
        file: String,
        /// 남길 페이지(1부터). 예: 1-3,5,8-10
        range: String,
        /// 저장 경로(생략 시 원본 옆에 _페이지 붙여 저장)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// 한글 깨진 파일(EUC-KR/CP949)을 UTF-8로 복구합니다. 예: wonjang 깨짐 가계부.csv
    #[command(aliases = ["깨짐", "한글복구"])]
    Encfix {
        /// 텍스트 파일 경로(.txt .csv 등)
        file: String,
        /// 저장 경로(생략 시 원본 옆에 _utf8 붙여 저장)
        #[arg(long = "출력")]
        output: Option<String>,
        /// 반대로 UTF-8 → CP949(옛 시스템 업로드용)
        #[arg(long = "되돌리기")]
        reverse: bool,
    },
    /// 받은편지함을 읽습니다(IMAP, 앱 비밀번호). 예: wonjang 메일 --안읽음
    #[command(alias = "메일")]
    Mail {
        /// 가져올 최근 메일 수(기본 10)
        #[arg(long = "개수", default_value_t = 10)]
        count: usize,
        /// 안 읽은 메일만 보기
        #[arg(long = "안읽음")]
        unseen: bool,
    },
    /// 메일을 보냅니다(SMTP, 파일 첨부 가능). 예: wonjang 메일보내기 --받는사람 a@b.com --제목 "안녕" --내용 "본문" --첨부 계약서.pdf
    #[command(name = "메일보내기", alias = "메일전송")]
    MailSend {
        /// 받는 사람 이메일 주소
        #[arg(long = "받는사람")]
        to: String,
        /// 제목
        #[arg(long = "제목")]
        subject: String,
        /// 본문 내용
        #[arg(long = "내용")]
        body: String,
        /// 첨부할 파일(여러 번 지정 가능)
        #[arg(long = "첨부")]
        attach: Vec<String>,
    },
    /// 받은편지함에서 보낸이·제목으로 메일을 찾습니다. 예: wonjang 메일검색 영수증
    #[command(name = "메일검색", alias = "메일찾기")]
    MailSearch {
        /// 검색어(보낸이 또는 제목에 포함)
        query: String,
        /// 검색할 최근 메일 범위(기본 100통)
        #[arg(long = "최근", default_value_t = 100)]
        scan: usize,
    },
    /// 메일의 첨부파일을 저장합니다. 예: wonjang 메일첨부 1 --저장폴더 ~/Downloads
    #[command(name = "메일첨부", alias = "메일첨부저장")]
    MailAttach {
        /// 몇 번째 최신 메일인지(1=가장 최근, 기본 1)
        #[arg(default_value_t = 1)]
        num: usize,
        /// 저장할 폴더(생략 시 현재 폴더)
        #[arg(long = "저장폴더")]
        dir: Option<String>,
        /// 안 읽은 메일 중에서 고르기
        #[arg(long = "안읽음")]
        unseen: bool,
    },
    /// 특정 메일의 본문을 읽습니다. 예: wonjang 메일읽기 1 (가장 최근)
    #[command(name = "메일읽기", alias = "메일내용")]
    MailRead {
        /// 몇 번째 최신 메일인지(1=가장 최근, 기본 1)
        #[arg(default_value_t = 1)]
        num: usize,
        /// 안 읽은 메일 중에서 고르기
        #[arg(long = "안읽음")]
        unseen: bool,
    },
    /// 내 기록을 자랑 카드 한 장으로(습관 잔디·집중·지출·D-day). 예: wonjang 자랑 (주간은 --주)
    #[command(alias = "자랑")]
    Brag {
        /// 특정 달(YYYY-MM, 기본 이번 달)
        #[arg(long = "달")]
        month: Option<String>,
        /// 주간 카드(이번 주 + 지난주 대비 ▲▼)
        #[arg(long = "주")]
        week: bool,
        /// 박스 폭(기본 46, 카톡엔 34 권장)
        #[arg(long = "폭", default_value_t = 46)]
        width: usize,
        /// 색 없이 출력(파이프·복붙용)
        #[arg(long = "no-color")]
        no_color: bool,
        /// 카드를 클립보드에 복사(카톡·메모에 바로 붙여넣기)
        #[arg(long = "복사")]
        copy: bool,
    },
    /// 비서 현황을 한눈에 봅니다(약속·할일·디데이·예약작업).
    #[command(alias = "현황")]
    Status,
    /// 원장이 할 수 있는 일을 카테고리별로 안내합니다.
    #[command(alias = "도움")]
    Guide,
    /// 원장의 성격(말투·태도)을 고릅니다. 예: wonjang 성격 친구
    #[command(alias = "성격")]
    Soul {
        /// 프리셋 이름(기본/친구/집사/선배/발랄) 또는 '초기화'. 생략 시 현재·목록
        preset: Option<String>,
    },
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
    #[command(aliases = ["지출", "가계부"])]
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
    #[command(aliases = ["집중", "타이머"])]
    Focus {
        /// 집중 시간(분). 생략하면 오늘 집중 요약.
        minutes: Option<i64>,
        /// 무엇에 집중하는지(선택)
        #[arg(trailing_var_arg = true)]
        label: Vec<String>,
    },
    /// 즐겨찾기 관리(사이트/폴더/앱 단축어).
    #[command(aliases = ["즐겨찾기", "북마크", "즐찾"])]
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
    #[command(aliases = ["환율", "환전"])]
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
    #[command(aliases = ["평", "평수"])]
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
    /// 전월세 전환 계산(전세→반월세 월세). 예: wonjang 전월세 30000 5.5 10000
    #[command(aliases = ["전월세", "반전세"])]
    Jeonse {
        /// 전세보증금(만 원 단위). 예: 30000 = 3억
        jeonse: f64,
        /// 전월세전환율(%, 법정 상한=기준금리+2%). 예: 5.5
        rate: f64,
        /// 월세 보증금(만 원, 생략 시 0=순수 월세)
        #[arg(default_value_t = 0.0)]
        deposit: f64,
    },
    /// 법정 퇴직금 추정(근로기준법). 예: wonjang 퇴직금 300 3 6
    #[command(alias = "퇴직금")]
    Severance {
        /// 월 평균임금(만 원 단위). 예: 300 = 300만 원
        monthly: f64,
        /// 근속 연수. 예: 3
        years: u32,
        /// 근속 개월(0~11, 생략 시 0)
        #[arg(default_value_t = 0)]
        months: u32,
    },
    /// 연차 휴가 일수(근로기준법). 예: wonjang 연차 5
    #[command(alias = "연차")]
    AnnualLeave {
        /// 근속 연수. 예: 5
        years: u32,
        /// 근속 개월(0~11, 1년 미만일 때만 의미, 생략 시 0)
        #[arg(default_value_t = 0)]
        months: u32,
    },
    /// 자동차세 계산(비영업 승용). 예: wonjang 자동차세 1998 3
    #[command(alias = "자동차세")]
    CarTax {
        /// 배기량(cc). 예: 1998
        cc: u32,
        /// 차령(년, 생략 시 0=신차). 3년부터 경감
        #[arg(default_value_t = 0)]
        age: u32,
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
    /// 단위 변환(온도·무게·길이·속도·부피·넓이). 예: wonjang 변환 100 c
    #[command(alias = "변환")]
    Convert {
        /// 값
        value: f64,
        /// 단위(c/f · kg/lb · cm/inch · km/mile · kmh/mph · l/gal · sqm/sqft)
        unit: String,
    },
    /// BMI 계산(아시아 기준 판정). 예: wonjang bmi 175 68
    Bmi {
        /// 키(cm)
        height: f64,
        /// 몸무게(kg)
        weight: f64,
    },
    /// 수면 시간 계산(90분 주기). 예: wonjang 수면 07:00 (없으면 지금 자면)
    #[command(alias = "수면")]
    Sleep {
        /// 기상 시각(HH:MM). 생략 시 지금 자는 기준 기상 시각 추천
        wake: Option<String>,
    },
    /// 기초대사량·하루 권장 칼로리. 예: wonjang 칼로리 남 30 175 70
    #[command(alias = "칼로리")]
    Calorie {
        /// 성별(남/여)
        sex: String,
        /// 나이
        age: u32,
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
    /// 글자수 세기(공백 포함/제외, 자소서 제한 체크). 예: wonjang 글자수 "자소서" --제한 1000
    #[command(alias = "글자수")]
    Chars {
        /// 셀 텍스트(여러 단어면 공백으로 이어 붙여 셈)
        text: Vec<String>,
        /// 글자수 제한(자소서 등) — 남은/초과 글자를 함께 표시
        #[arg(long = "제한")]
        limit: Option<usize>,
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
    /// 로마 숫자 변환(숫자↔로마자). 예: wonjang 로마 2024 / wonjang 로마 MMXXIV
    #[command(alias = "로마")]
    Roman {
        /// 정수(1~3999) 또는 로마 숫자
        value: String,
    },
    /// 세계 시간(주요 도시 현재 시각, DST 반영). 예: wonjang 세계시간 [뉴욕]
    #[command(alias = "세계시간")]
    Worldtime {
        /// 도시 검색어(서울/뉴욕/런던…). 생략 시 전체
        city: Option<String>,
    },
    /// 시간대 변환(도시 간). 예: wonjang 시차 09:00 서울 뉴욕
    #[command(alias = "시차")]
    Tzconv {
        /// 변환할 시각(HH:MM)
        time: String,
        /// 출발 도시
        from: String,
        /// 도착 도시
        to: String,
    },
    /// 유닉스 타임스탬프 변환. 예: wonjang 타임스탬프 1700000000 (없으면 현재)
    #[command(alias = "타임스탬프")]
    Timestamp {
        /// 유닉스 초/밀리초 또는 날짜(YYYY-MM-DD). 생략 시 현재 시각
        value: Option<String>,
    },
    /// base64/URL 인코딩·디코딩. 예: wonjang 인코딩 base64 "hello"
    #[command(alias = "인코딩")]
    Encode {
        /// 방식: base64 또는 url
        method: String,
        /// 대상 텍스트
        text: Vec<String>,
        /// 디코딩(미지정 시 인코딩)
        #[arg(short = 'd', long = "디코드")]
        decode: bool,
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
    /// 간단 일기 기록/보기. 예: wonjang 일기 "오늘 있었던 일" (없으면 이번 달 보기)
    #[command(alias = "일기")]
    Journal {
        /// 기록할 내용(생략 시 이번 달 일기 보기)
        text: Vec<String>,
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
    /// 내 공인 IP·통신사·위치 확인. 예: wonjang 내아이피
    #[command(alias = "내아이피")]
    Myip,
    /// 사이트/서버 상태 확인(살아있나·응답속도). 예: wonjang 사이트 example.com
    #[command(alias = "사이트")]
    Uptime {
        /// 점검할 URL(http(s):// 생략 가능)
        url: String,
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
    /// 날짜 계산(며칠째·기념일 / D-day / 두 날짜 사이 / N일 후). 예: wonjang 날짜 2024-01-01
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
    /// 색상 변환(HEX↔RGB↔HSL). 예: wonjang 색 #ff5733 / wonjang 색 255 87 51
    #[command(alias = "색")]
    Color {
        /// 헥스(#ff5733) 또는 R G B(255 87 51)
        input: Vec<String>,
    },
    /// UUID v4 생성(무작위). 예: wonjang uuid -n 3
    Uuid {
        /// 생성 개수(기본 1)
        #[arg(short = 'n', long = "개수", default_value_t = 1)]
        count: usize,
    },
    /// 안전한 비밀번호 생성(OS 난수). 예: wonjang 비번 16 --기호
    #[command(alias = "비번")]
    Password {
        /// 길이(기본 16, 4~128)
        length: Option<usize>,
        /// 특수기호 포함
        #[arg(long = "기호")]
        symbols: bool,
        /// 생성 개수(기본 1)
        #[arg(short = 'n', long = "개수", default_value_t = 1)]
        count: usize,
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
    /// 모든 가계부 기록을 CSV로 내보냅니다(엑셀·wonjang 엑셀로 월별·분류별 분석). 예: wonjang 지출 내보내기
    #[command(name = "내보내기", alias = "csv")]
    Export {
        /// 저장 경로(생략 시 다운로드 폴더의 가계부.csv)
        #[arg(long = "출력")]
        output: Option<String>,
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
    /// 디데이들을 캘린더(.ics) 파일로 — 구글·애플 캘린더에 '가져오기'로 넣습니다.
    #[command(name = "내보내기", alias = "ics")]
    Export {
        /// 저장 경로(생략 시 다운로드 폴더의 디데이.ics)
        #[arg(long = "출력")]
        output: Option<String>,
    },
    /// 디데이를 공유용 카드 한 장으로(카톡·SNS 캡처). 예: wonjang 디데이 카드 수능
    #[command(name = "카드", alias = "card")]
    Card {
        /// 디데이 이름(생략 시 가장 가까운 디데이)
        name: Option<String>,
        /// 박스 폭(기본 40, 카톡엔 34 권장)
        #[arg(long = "폭", default_value_t = 40)]
        width: usize,
        /// 색 없이 출력(파이프·복붙용)
        #[arg(long = "no-color")]
        no_color: bool,
        /// 카드를 클립보드에 복사(카톡·메모에 바로 붙여넣기)
        #[arg(long = "복사")]
        copy: bool,
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
            group,
            sheet,
            filter,
            sort,
            ascending,
            save,
            rows,
            json,
        }) => {
            return cmd_excel(
                file,
                column.as_deref(),
                group.as_deref(),
                sheet.as_deref(),
                filter.as_deref(),
                sort.as_deref(),
                *ascending,
                save.as_deref(),
                *rows,
                *json,
            )
        }
        Some(Commands::SheetMerge {
            files,
            save,
            source,
        }) => return cmd_merge_tables(files, save.as_deref(), *source),
        Some(Commands::Image {
            files,
            width,
            scale,
            quality,
            format,
            output,
        }) => {
            return cmd_image(
                files,
                *width,
                *scale,
                *quality,
                format.as_deref(),
                output.as_deref(),
            )
        }
        Some(Commands::Brag {
            month,
            week,
            width,
            no_color,
            copy,
        }) => return cmd_brag(month.as_deref(), *week, *width, *no_color, *copy),
        Some(Commands::Mail { count, unseen }) => return cmd_mail(*count, *unseen),
        Some(Commands::MailRead { num, unseen }) => return cmd_mail_read(*num, *unseen),
        Some(Commands::MailAttach { num, dir, unseen }) => {
            return cmd_mail_attach(*num, dir.as_deref(), *unseen)
        }
        Some(Commands::MailSearch { query, scan }) => return cmd_mail_search(query, *scan),
        Some(Commands::MailSend {
            to,
            subject,
            body,
            attach,
        }) => return cmd_mail_send(to, subject, body, attach),
        Some(Commands::Encfix {
            file,
            output,
            reverse,
        }) => return cmd_encfix(file, output.as_deref(), *reverse),
        Some(Commands::PhotosPdf { files, output }) => {
            return cmd_photos_pdf(files, output.as_deref())
        }
        Some(Commands::ImageStitch {
            files,
            vertical,
            horizontal,
            output,
        }) => {
            // --세로(명시)는 기본이므로 --가로만 가로로. 둘 다면 --세로 우선.
            return cmd_image_stitch(files, *horizontal && !*vertical, output.as_deref());
        }
        Some(Commands::PdfMerge { files, output }) => {
            return cmd_pdf_merge(files, output.as_deref())
        }
        Some(Commands::PdfEncrypt {
            file,
            password,
            output,
        }) => return cmd_pdf_encrypt(file, password, output.as_deref()),
        Some(Commands::PdfRotate {
            file,
            angle,
            pages,
            output,
        }) => return cmd_pdf_rotate(file, *angle, pages.as_deref(), output.as_deref()),
        Some(Commands::PdfPages {
            file,
            range,
            output,
        }) => return cmd_pdf_pages(file, range, output.as_deref()),
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
        Some(Commands::ZipView { file }) => return cmd_zip_view(file),
        Some(Commands::Rename {
            path,
            find,
            replace,
            run,
        }) => return cmd_rename(path, find, replace, *run),
        Some(Commands::Soul { preset }) => return cmd_soul(preset.as_deref()),
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
        Some(Commands::Jeonse {
            jeonse,
            rate,
            deposit,
        }) => return cmd_jeonse(*jeonse, *rate, *deposit),
        Some(Commands::Severance {
            monthly,
            years,
            months,
        }) => return cmd_severance(*monthly, *years, *months),
        Some(Commands::AnnualLeave { years, months }) => return cmd_annual_leave(*years, *months),
        Some(Commands::CarTax { cc, age }) => return cmd_car_tax(*cc, *age),
        Some(Commands::Menu { category }) => return cmd_menu(category.as_deref()),
        Some(Commands::Dutch {
            total,
            people,
            unit,
        }) => return cmd_dutch(*total, *people, *unit),
        Some(Commands::Convert { value, unit }) => return cmd_convert(*value, unit),
        Some(Commands::Bmi { height, weight }) => return cmd_bmi(*height, *weight),
        Some(Commands::Sleep { wake }) => return cmd_sleep(wake.as_deref()),
        Some(Commands::Calorie {
            sex,
            age,
            height,
            weight,
        }) => return cmd_calorie(sex, *age, *height, *weight),
        Some(Commands::Discount { price, rates }) => return cmd_discount(*price, rates),
        Some(Commands::Vat { amount }) => return cmd_vat(*amount),
        Some(Commands::Chars { text, limit }) => return cmd_chars(text, *limit),
        Some(Commands::Choseong { text }) => return cmd_choseong(text),
        Some(Commands::Keystroke { text }) => return cmd_keystroke(text),
        Some(Commands::Hanstroke { text }) => return cmd_hanstroke(text),
        Some(Commands::Amount { value }) => return cmd_amount(*value),
        Some(Commands::Calc { expr }) => return cmd_calc(expr),
        Some(Commands::Time { items }) => return cmd_time(items),
        Some(Commands::Radix { value }) => return cmd_radix(value),
        Some(Commands::Roman { value }) => return cmd_roman(value),
        Some(Commands::Worldtime { city }) => return cmd_worldtime(city.as_deref()),
        Some(Commands::Tzconv { time, from, to }) => return cmd_tzconv(time, from, to),
        Some(Commands::Timestamp { value }) => return cmd_timestamp(value.as_deref()),
        Some(Commands::Encode {
            method,
            text,
            decode,
        }) => return cmd_encode(method, text, *decode),
        Some(Commands::Wage {
            hourly,
            weekly_hours,
        }) => return cmd_wage(*hourly, *weekly_hours),
        Some(Commands::Holiday { year }) => return cmd_holiday(*year),
        Some(Commands::Journal { text }) => return cmd_journal(text),
        Some(Commands::Congestion { area }) => return cmd_congestion(&cfg, area),
        Some(Commands::Geeknews { count }) => return cmd_geeknews(*count),
        Some(Commands::Qr {
            text,
            wifi,
            password,
        }) => return cmd_qr(text, wifi.as_deref(), password),
        Some(Commands::Github { slug }) => return cmd_github(slug),
        Some(Commands::Myip) => return cmd_myip(),
        Some(Commands::Uptime { url }) => return cmd_uptime(url),
        Some(Commands::Bike { query }) => return cmd_bike(&cfg, query.as_deref()),
        Some(Commands::Date { from, to, plus }) => {
            return cmd_date(from.as_deref(), to.as_deref(), *plus)
        }
        Some(Commands::Color { input }) => return cmd_color(input),
        Some(Commands::Uuid { count }) => return cmd_uuid(*count),
        Some(Commands::Password {
            length,
            symbols,
            count,
        }) => return cmd_password(*length, *symbols, *count),
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
    // (연결 정보는 REPL 배너 / 단발 위임 시점에 표시되므로 여기선 생략)
    let backend = engine::resolve(&cfg)?;
    let eng = build_engine(backend, &cfg);

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
    // `preset run` 외에, 첫 단어가 프리셋 이름이면(예: `wonjang 일지 오늘…`) 그 프리셋으로
    // 해석한다 — 도움말이 `일지/메모 (프리셋)`처럼 직접 호출을 암시하는데 실제로는
    // 일반 AI로 새던 걸 메운다.
    let one_shot = preset_prompt.unwrap_or_else(|| match resolve_bare_preset(&cli.prompt) {
        Some((full, note)) => {
            ui::note(&note);
            full
        }
        None => cli.prompt.join(" "),
    });
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
    ui::banner(&eng.label(cfg), eng.backend_ready());
    ui::onboarding_if_first();

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
            s if s.starts_with("/성격") => {
                let arg = s["/성격".len()..].trim();
                if arg.is_empty() {
                    ui::info(&format!(
                        "현재 성격: {}",
                        soul::active_persona().chars().take(34).collect::<String>()
                    ));
                    ui::info("바꾸기: /성격 친구|집사|선배|발랄|기본  ('초기화'로 기본 복귀)");
                } else {
                    let res = if arg == "초기화" || arg == "기본" {
                        soul::reset()
                    } else {
                        soul::set_preset(arg)
                    };
                    match res {
                        Ok(()) => {
                            // 새 성격을 즉시 반영하도록 시스템 프롬프트 재구성.
                            if let (Ok(mem), Ok(skills)) =
                                (memory::Memory::load(), skill::SkillStore::load())
                            {
                                if !messages.is_empty() {
                                    messages[0] = Message::system(agent::system_prompt(
                                        mem.prompt_block(),
                                        skills.prompt_block(),
                                    ));
                                    sess.save(messages).ok();
                                }
                            }
                            ui::info(&format!("이제부터 '{arg}' 성격으로 말할게요. 🎭"));
                        }
                        Err(e) => ui::error(&format!("{e}")),
                    }
                }
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
         /성격     원장 말투 바꾸기 (친구·집사·선배·발랄·기본)\n  \
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

/// 선제 알림(브리핑·공휴일·습관·주간결산)의 '보낸 날' 표시 — 파일에 저장해 데몬 재시작에도 중복 방지.
#[derive(Default, Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
struct AlertState {
    #[serde(default)]
    last_briefed: Option<String>,
    #[serde(default)]
    last_holiday: Option<String>,
    #[serde(default)]
    last_habit: Option<String>,
    #[serde(default)]
    last_weekly: Option<String>,
}

impl AlertState {
    fn path() -> Option<std::path::PathBuf> {
        dirs::data_dir().map(|d| d.join("wonjang").join("alert_state.json"))
    }
    fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self) {
        if let (Some(p), Ok(json)) = (Self::path(), serde_json::to_string_pretty(self)) {
            let _ = crate::util::atomic_write(&p, json.as_bytes());
        }
    }
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
    // 선제 알림 dedup 상태를 파일에서 복원(재시작해도 같은 윈도우 중복 푸시 방지).
    let mut alerts = AlertState::load();
    let mut alerts_saved = alerts.clone();
    // 디스크 저장이 실패해도(읽기전용·디스크 가득) 같은 작업을 매 틱 재실행(LLM·푸시 폭주)하지
    // 않도록, 실행 시각을 인메모리에도 보관해 매 틱 reload된 store에 병합한다.
    let mut ran_at: std::collections::HashMap<u64, u128> = std::collections::HashMap::new();
    // 시세 알림도 같은 결: 저장 실패해도 매 틱 재발동하지 않게 인메모리로 발동 워치 ID 보관.
    let mut watch_fired: std::collections::HashSet<u64> = std::collections::HashSet::new();

    loop {
        // 매 틱마다 저장소를 다시 읽어 추가/삭제를 반영한다.
        // 일시적 읽기 실패(동시 쓰기 중 파일 교체·I/O 순간 오류 등)로 데몬 전체가
        // 죽으면 모든 알림이 조용히 멈춘다 → 로그만 남기고 이번 회차를 건너뛴다.
        let mut store = match cron::CronStore::load() {
            Ok(s) => s,
            Err(e) => {
                ui::error(&format!(
                    "예약 작업 목록을 읽지 못했어요(이번 회차 건너뜀): {e:#}"
                ));
                tokio::time::sleep(tick).await;
                continue;
            }
        };
        // 인메모리 실행기록을 reload된 store에 병합(디스크 저장 실패분 보존 → 재실행 폭주 방지).
        for t in store.tasks.iter_mut() {
            if let Some(&ms) = ran_at.get(&t.id) {
                if t.last_run_ms.is_none_or(|d| d < ms) {
                    t.last_run_ms = Some(ms);
                }
            }
        }
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

            // 준비 단계 읽기 실패도 데몬을 죽이지 않고 이 작업만 건너뛴다(다음 틱에 재시도).
            let mem = match memory::Memory::load() {
                Ok(m) => m,
                Err(e) => {
                    ui::error(&format!("작업 #{id} 준비 실패(메모리 읽기, 건너뜀): {e:#}"));
                    continue;
                }
            };
            let skills = match skill::SkillStore::load() {
                Ok(s) => s,
                Err(e) => {
                    ui::error(&format!("작업 #{id} 준비 실패(스킬 읽기, 건너뜀): {e:#}"));
                    continue;
                }
            };
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

            // 실행 시각 기록(디스크 + 인메모리 둘 다 — 디스크 저장 실패해도 재실행 방지).
            let ran_ms = cron::now_ms();
            if let Some(t) = store.tasks.iter_mut().find(|t| t.id == id) {
                t.last_run_ms = Some(ran_ms);
            }
            ran_at.insert(id, ran_ms);
            store.save().ok();
        }

        // 약속·알림 확인: 때가 된 알림을 데스크탑 알림 + 푸시 채널로 띄운다.
        check_due_reminders(cfg);

        // 매일 자동 브리핑(설정된 시각이 지났고 오늘 아직 안 보냈으면).
        maybe_send_briefing(eng, cfg, &ctx, &mut alerts.last_briefed).await;

        // 코인 시세 알림: 목표가에 도달한 알림을 푸시한다.
        check_price_watches(cfg, &mut watch_fired).await;

        // 공휴일 전날이면 "내일 빨간날" 알림(선제성).
        maybe_alert_holiday_eve(cfg, &mut alerts.last_holiday).await;

        // 저녁이면 끊길 위기 습관을 먼저 챙겨 리마인드(선톡).
        maybe_alert_habit_evening(cfg, &mut alerts.last_habit).await;

        // 일요일 저녁이면 이번 주 결산을 먼저 푸시(자랑 카드 트리거).
        maybe_push_weekly_recap(cfg, &mut alerts.last_weekly).await;

        // 알림 dedup 표시가 바뀌었으면 파일에 저장(재시작 후 중복 방지).
        if alerts != alerts_saved {
            alerts.save();
            alerts_saved = alerts.clone();
        }

        tokio::time::sleep(tick).await;
    }
}

/// 목표가에 도달한 시세 알림을 푸시하고 발동 표시한다(코인 + 환율).
async fn check_price_watches(cfg: &Config, fired: &mut std::collections::HashSet<u64>) {
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
            // 디스크 저장 실패해도 같은 워치를 매 틱 재발동(푸시 폭주)하지 않도록 인메모리 dedup.
            if !fired.contains(&w.id) && watch::should_trigger(w, p) {
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
                fired.insert(w.id);
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

/// 공휴일 전날이면 "내일 빨간날" 알림을 먼저 보낸다(헤르메스식 선제성).
/// 푸시 채널이 설정돼 있을 때만, 하루 한 번, 아침(9시) 이후에 점검한다.
async fn maybe_alert_holiday_eve(cfg: &Config, last_alert: &mut Option<String>) {
    use chrono::{Datelike, Timelike};
    if push::configured_channels(cfg).is_empty() {
        return;
    }
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    // 하루 한 번, 아침 9시 이후에만.
    if last_alert.as_deref() == Some(today.as_str()) || now.hour() < 9 {
        return;
    }
    let tomorrow = now.date_naive() + chrono::Duration::days(1);
    // 내일이 속한 연도를 조회(연말 12/31에 다음 해 1/1 공휴일을 놓치지 않게).
    if let Ok(list) = holidays::fetch(tomorrow.year()).await {
        // 조회에 성공했을 때만 오늘 점검 완료로 표시(네트워크 실패 시 같은 날 재시도).
        *last_alert = Some(today.clone());
        if let Some(h) = list.iter().find(|h| h.date == tomorrow) {
            let msg = format!(
                "🔴 내일({})은 '{}'이에요! 빨간날 잘 보내세요 🎉",
                tomorrow.format("%-m월 %-d일"),
                h.name
            );
            let sent = push::push(cfg, &msg).await;
            ui::note(&format!("공휴일 전날 알림 전송({sent}개 채널)."));
        }
    }
}

/// 끊길 위기 습관 목록(streak, 이름; streak 내림차순) → 저녁 리마인드 문구(없으면 None).
fn habit_evening_nudge(at_risk: &[(i64, String)]) -> Option<String> {
    match at_risk {
        [] => None,
        [(streak, name)] => Some(format!(
            "🔥 '{name}' {streak}일 연속 중이에요! 자기 전에 오늘치 체크해요."
        )),
        [(streak, name), rest @ ..] => Some(format!(
            "🔥 '{name}'({streak}일 연속) 외 {}개 습관이 오늘 아직이에요. 자기 전에 챙겨봐요!",
            rest.len()
        )),
    }
}

/// 저녁에 "끊길 위기 습관"(streak≥2·오늘 미완)을 먼저 챙겨 푸시한다(헤르메스식 선톡).
/// 하루 한 번, 저녁 8시 이후, 푸시 채널이 설정돼 있을 때만.
async fn maybe_alert_habit_evening(cfg: &Config, last_alert: &mut Option<String>) {
    use chrono::Timelike;
    if push::configured_channels(cfg).is_empty() {
        return;
    }
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    if last_alert.as_deref() == Some(today.as_str()) || now.hour() < 20 {
        return;
    }
    let habit = match habits::HabitStore::load() {
        Ok(h) => h,
        Err(_) => return,
    };
    let today_d = ddays::today();
    let today_s = habits::today_str();
    let mut at_risk: Vec<(i64, String)> = habit
        .items
        .iter()
        .filter(|h| !h.done_today(&today_s))
        .map(|h| (h.streak(today_d), h.name.clone()))
        .filter(|(s, _)| *s >= 2)
        .collect();
    // 보낼 게 없어도 하루 한 번만 시도(스팸 방지).
    *last_alert = Some(today.clone());
    at_risk.sort_by_key(|b| std::cmp::Reverse(b.0));
    if let Some(msg) = habit_evening_nudge(&at_risk) {
        let sent = push::push(cfg, &msg).await;
        ui::note(&format!("저녁 습관 리마인드 전송({sent}개 채널)."));
    }
}

/// 일요일 저녁, 이번 주 결산을 먼저 푸시한다(자랑 카드 보러 가게 유도 — 선톡 × 전염).
/// 일요일 19시 이후·주 1회·푸시 채널 설정 시. 데이터 집계는 cmd_brag_weekly와 동일.
async fn maybe_push_weekly_recap(cfg: &Config, last: &mut Option<String>) {
    use chrono::{Datelike, Timelike, Weekday};
    if push::configured_channels(cfg).is_empty() {
        return;
    }
    let now = chrono::Local::now();
    if now.weekday() != Weekday::Sun || now.hour() < 19 {
        return;
    }
    let today = now.format("%Y-%m-%d").to_string();
    if last.as_deref() == Some(today.as_str()) {
        return;
    }
    *last = Some(today.clone()); // 데이터가 없어 안 보내도 주 1회만 시도.

    let td = ddays::today();
    let day = |d: chrono::NaiveDate| d.format("%Y-%m-%d").to_string();
    let habit_store = habits::HabitStore::load().unwrap_or_default();
    let streak = habit_store
        .items
        .iter()
        .max_by_key(|h| h.streak(td))
        .map(|h| (h.name.clone(), h.streak(td)));
    let foc = focus::FocusStore::load().unwrap_or_default();
    let exp = expenses::ExpenseStore::load().unwrap_or_default();
    let (mut tw_f, mut lw_f, mut tw_e, mut lw_e) = (0i64, 0i64, 0i64, 0i64);
    for i in 0..7 {
        tw_f += foc.today_total(&day(td - chrono::Duration::days(i)));
        lw_f += foc.today_total(&day(td - chrono::Duration::days(i + 7)));
        tw_e += exp.total_on(&day(td - chrono::Duration::days(i)));
        lw_e += exp.total_on(&day(td - chrono::Duration::days(i + 7)));
    }
    if let Some(msg) = card::weekly_recap_text(streak, tw_f, tw_f - lw_f, tw_e, tw_e - lw_e) {
        let sent = push::push(cfg, &msg).await;
        ui::note(&format!("주간 결산 선톡 전송({sent}개 채널)."));
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
    // 데몬이 꺼져 있던 동안 밀린 알림이 첫 실행에 한꺼번에 폭주하지 않도록:
    // 너무 오래 지난 알림은 조용히 처리만 하고 울리지 않는다(이미 지난 일을 무더기로
    // 알리면 스팸 → 첫인상 악화). 반복 알림은 handle_fired가 다음 회차로 재예약하므로
    // 앞으로의 알림엔 영향 없다. 판정은 reminders::should_alert(테스트로 고정).
    for r in &due {
        if reminders::should_alert(r.at_unix, now) {
            ui::note(&format!("🔔 알림: {}", r.title));
            reminders::desktop_notify("원장 알림 🔔", &r.title);
            // 설정된 채널(카카오/디스코드/슬랙/텔레그램)로도 푸시 → 외출 중에도 받음.
            push::push_blocking(cfg, &format!("🔔 {}", r.title));
        }
        // 반복이면 다음 회차로 재예약, 아니면 완료 표시(스팸 방지를 위해 stale도 처리).
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
            "디스코드: WONJANG_DISCORD_WEBHOOK 에 웹훅 URL,\n  \
             슬랙: WONJANG_SLACK_WEBHOOK 에 Incoming Webhook URL,\n  \
             텔레그램: 토큰 + telegram_allowed_ids 중 하나를 설정하세요.",
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
                ("wonjang 세계시간 [도시]", "주요 도시 현재 시각(DST)"),
                ("wonjang 시차 09:00 서울 뉴욕", "도시 간 시간대 변환"),
                ("wonjang 긱뉴스 [개수]", "개발·기술·스타트업 뉴스"),
                ("wonjang 깃헙 <owner/repo>", "GitHub 저장소 정보"),
                ("wonjang 내아이피", "공인 IP·통신사·위치"),
                ("wonjang 사이트 <url>", "사이트 상태·응답속도 점검"),
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
                ("wonjang 디데이 카드 [이름]", "D-day 공유 카드(카톡 캡처)"),
                ("wonjang 디데이 내보내기", "디데이를 캘린더(.ics)로"),
                ("wonjang 집중 <분> [무엇]", "뽀모도로 타이머"),
            ],
        ),
        (
            "📒 기록 & 지식",
            &[
                ("wonjang 지출 add <금액> <분류>", "가계부"),
                ("wonjang 지출 내보내기", "가계부를 CSV로(엑셀 분석)"),
                ("wonjang 습관 done <이름>", "습관 트래커(연속일수)"),
                ("wonjang 일기 \"<내용>\"", "간단 일기(월별 저장)"),
                ("wonjang 일지/메모 (프리셋)", "옵시디언 노트"),
                ("wonjang notion search \"...\"", "노션 검색/기록"),
            ],
        ),
        (
            "📲 알림 & 편의",
            &[
                (
                    "wonjang notify \"메시지\"",
                    "카카오/디스코드/슬랙/텔레그램 푸시",
                ),
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
                (
                    "wonjang 전월세 <전세금> <전환율%> [보증금]",
                    "전세→월세 전환",
                ),
                ("wonjang 퇴직금 <월급> <근속년> [개월]", "법정 퇴직금 추정"),
                ("wonjang 연차 <근속년> [개월]", "연차 휴가 일수(근로기준법)"),
                ("wonjang 자동차세 <cc> [차령]", "자동차세(비영업 승용)"),
                ("wonjang 할인 <원가> <%>...", "할인가(중복 할인)"),
                ("wonjang 부가세 <금액>", "공급가/세액 분리"),
                ("wonjang 나이 <YYYY-MM-DD>", "만 나이·살아온 날·기념일"),
                (
                    "wonjang 날짜 <날짜> [날짜2]",
                    "며칠째·기념일·D-day·두 날짜 사이",
                ),
                ("wonjang 평 <숫자>", "평↔㎡ 변환"),
                (
                    "wonjang 변환 <값> <단위>",
                    "온도/무게/길이 + 돈·근·관(한국)",
                ),
                ("wonjang bmi <키> <몸무게>", "BMI(아시아 기준)"),
                (
                    "wonjang 칼로리 <성별> <나이> <키> <몸무게>",
                    "기초대사량·권장칼로리",
                ),
                ("wonjang 수면 [기상시각]", "수면 시간(90분 주기)"),
                ("wonjang 더치 <총액> <인원>", "더치페이(n빵)"),
                ("wonjang 뽑기 <후보들>", "제비뽑기/추첨"),
                ("wonjang 메뉴 [카테고리]", "오늘 뭐 먹지?"),
                (
                    "wonjang 글자수 \"<텍스트>\" [--제한 N]",
                    "자소서 글자수·제한 대비 남은 글자",
                ),
                ("wonjang 초성 \"<텍스트>\"", "한글 초성 추출"),
                ("wonjang 영타 \"<한글>\"", "한글→영문 타자(dkssud)"),
                ("wonjang 한타 <영문>", "영문→한글 복원(잘못 친 글자)"),
                ("wonjang 금액 <숫자>", "한글 금액(계약서·수표)"),
                ("wonjang 계산 \"<식>\"", "사칙연산 계산기"),
                ("wonjang 시간 09:00 + 8:30", "시간 더하기/빼기"),
                ("wonjang 진법 255", "2/8/10/16진수 변환"),
                ("wonjang 타임스탬프 [값]", "유닉스 시각 ↔ 날짜"),
                ("wonjang 로마 2024", "로마 숫자 ↔ 숫자"),
                ("wonjang 인코딩 base64 <텍스트>", "base64/URL 인코딩·디코딩"),
                ("wonjang 비번 [길이] --기호", "안전한 비밀번호 생성"),
                ("wonjang uuid [-n N]", "UUID v4 생성"),
                ("wonjang 색 #ff5733", "HEX↔RGB↔HSL 색상 변환"),
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
                (
                    "wonjang 엑셀 <파일> --그룹 지점 --열 매출",
                    "분류별 묶어 집계(피벗)",
                ),
                ("wonjang 엑셀 <파일> --정렬 매출", "그 열로 정렬해 상위 행"),
                (
                    "wonjang 엑셀 <파일> --필터 지역=서울",
                    "조건 행만(정렬·집계와 조합)",
                ),
                (
                    "wonjang 엑셀 <파일.xlsx> --시트 재고",
                    "엑셀 다중 시트에서 선택",
                ),
                (
                    "wonjang 엑셀 <파일> --필터 … --저장 결과.csv",
                    "분석 결과를 새 CSV로 저장",
                ),
                (
                    "wonjang 표합치기 1월.csv 2월.csv --저장 합본.csv",
                    "월별·지점별 표 여러 개를 하나로",
                ),
                ("wonjang 또간집 <지역>", "풍자 또간집 선정 맛집(지역)"),
                ("wonjang 용량 [폴더]", "큰 파일·폴더 찾기(용량 분석)"),
                ("wonjang 중복 [폴더]", "내용 같은 중복 파일 찾기"),
                ("wonjang 정리 <폴더>", "종류별 자동 분류(미리보기→--실행)"),
                ("wonjang 이름변경 <폴더> A B", "파일명 A를 B로 일괄 치환"),
                ("wonjang 압축 <폴더>", "zip 압축 / 압축풀기 <zip>"),
                (
                    "wonjang 압축보기 <파일.zip>",
                    "풀지 않고 목록 보기(한글명 보정)",
                ),
                (
                    "wonjang 이미지 <사진들> --폭 1280",
                    "이미지 축소·압축(여러 장, --형식 jpg/png)",
                ),
                (
                    "wonjang 사진묶기 *.jpg",
                    "여러 사진을 PDF 한 파일로(서류 제출)",
                ),
                (
                    "wonjang 이미지이어붙이기 1.png 2.png --세로",
                    "여러 이미지를 한 장으로(긴 캡처·영수증)",
                ),
                (
                    "wonjang pdf합치기 a.pdf b.pdf",
                    "여러 PDF를 하나로(서류 합본)",
                ),
                (
                    "wonjang pdf페이지 <파일> 1-3,5",
                    "PDF에서 원하는 페이지만 추출",
                ),
                (
                    "wonjang pdf회전 <파일> 90",
                    "옆으로 스캔된 PDF 돌리기(90의 배수)",
                ),
                (
                    "wonjang pdf암호 <파일> --비번 …",
                    "PDF에 비밀번호 걸기(AES-256)",
                ),
                (
                    "wonjang 깨짐 <파일.csv>",
                    "한글 깨진 파일(CP949)→UTF-8 복구",
                ),
                ("wonjang 메일 --안읽음", "받은편지함 목록(IMAP·앱비밀번호)"),
                ("wonjang 메일읽기 1", "그 메일 본문 읽기(1=최신)"),
                ("wonjang 메일검색 <키워드>", "보낸이·제목으로 메일 찾기"),
                ("wonjang 메일첨부 1 --저장폴더 …", "메일 첨부파일 저장"),
                (
                    "wonjang 메일보내기 --받는사람 … --첨부 파일",
                    "메일 보내기(파일 첨부 가능)",
                ),
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
                ("wonjang 자랑 --복사", "회고 카드 → 클립보드(주간 --주)"),
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

/// zip을 풀지 않고 안의 목록만 보여준다(한글 파일명 깨짐 보정).
fn cmd_zip_view(file: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let zip_path = expand_path(file);
    if !zip_path.exists() {
        return Err(anyhow::anyhow!("zip 파일이 없어요: {}", zip_path.display()));
    }
    let f = std::fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| {
        anyhow::anyhow!("zip을 열지 못했어요({e}). 손상됐거나 zip이 아닐 수 있어요.")
    })?;
    let count = archive.len();
    println!();
    println!(
        "  📦 {}  ({}개 항목)",
        file.bright_cyan(),
        count.to_string().bright_white()
    );
    println!();
    let mut total_size = 0u64;
    let mut total_comp = 0u64;
    let mut files = 0usize;
    let mut dirs = 0usize;
    for i in 0..count {
        let entry = archive.by_index(i)?;
        let name = archive::decode_zip_name(entry.name_raw());
        if entry.is_dir() {
            dirs += 1;
            println!("     📁 {}", name.dimmed());
        } else {
            files += 1;
            let size = entry.size();
            total_size += size;
            total_comp += entry.compressed_size();
            println!("     📄 {}  {}", name, human_bytes(size).dimmed());
        }
    }
    println!();
    let ratio = total_comp
        .saturating_mul(100)
        .checked_div(total_size)
        .map(|comp_pct| 100u64.saturating_sub(comp_pct.min(100)))
        .unwrap_or(0);
    println!(
        "  파일 {files}개 · 폴더 {dirs}개 · 원본 {} → 압축 {} ({}% 절감)",
        human_bytes(total_size),
        human_bytes(total_comp),
        ratio
    );
    ui::info("     푸는 건: wonjang 압축풀기 <파일.zip>");
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

/// 통계 숫자를 깔끔히: 정수부엔 항상 천단위 콤마, 소수는 2자리까지(있을 때만).
fn fmt_stat_num(v: f64) -> String {
    let commas = |n: i64| expenses::won(n).trim_end_matches('원').to_string();
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return commas(v as i64);
    }
    // 소수도 정수부에 콤마를 넣어 큰 평균이 '1616666.67'처럼 안 보이게.
    let s = format!("{v:.2}"); // 반올림·자리올림은 여기서 정확히 처리됨
    let (sign, body) = s.strip_prefix('-').map_or(("", s.as_str()), |b| ("-", b));
    let (int_part, dec) = body.split_once('.').unwrap_or((body, "00"));
    match int_part.parse::<i64>() {
        Ok(n) => format!("{sign}{}.{dec}", commas(n)),
        Err(_) => s, // 지나치게 큰 값이면 원본 그대로(드묾)
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_excel(
    file: &str,
    column: Option<&str>,
    group: Option<&str>,
    sheet: Option<&str>,
    filter: Option<&str>,
    sort: Option<&str>,
    ascending: bool,
    save: Option<&str>,
    preview_rows: usize,
    json: bool,
) -> Result<()> {
    use owo_colors::OwoColorize;
    let mut table = sheet::Table::load_sheet(file, sheet)?;

    // 필터: 조건에 맞는 행만 남긴 새 표로 교체 → 이후 모든 분석이 부분집합 위에서.
    let filter_note = match filter {
        Some(expr) => {
            let before = table.rows.len();
            table = table.filtered(expr)?;
            Some((expr, before, table.rows.len()))
        }
        None => None,
    };

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
    // 다중 시트 엑셀이면 어떤 시트들이 있는지 알려준다(다른 탭 분석 유도).
    let sheets = sheet::list_sheets(file);
    if sheets.len() > 1 {
        let cur = sheet.unwrap_or(&sheets[0]);
        println!(
            "  {} 시트 {}개 [{}]: {} {}",
            "🗂".dimmed(),
            sheets.len(),
            cur.bright_white(),
            sheets.join(" · ").dimmed(),
            "(--시트 <이름>으로 전환)".dimmed()
        );
    }
    if let Some((expr, before, after)) = filter_note {
        println!(
            "  {} 필터 '{}': {before}행 중 {after}행",
            "🔎".dimmed(),
            expr.bright_white()
        );
        if after == 0 {
            println!("     {}", "조건에 맞는 행이 없어요.".dimmed());
            println!();
            return Ok(());
        }
    }

    // 결과 저장: --그룹이면 집계표를, 아니면 필터+정렬된 행을 새 CSV로.
    if let Some(out) = save {
        use std::path::Path;
        if Path::new(out) == Path::new(file) {
            anyhow::bail!("원본을 덮어쓸 수 없어요. 다른 경로로 저장하세요(--저장 다른이름.csv).");
        }
        let (out_headers, out_rows) = if let Some(gkey) = group {
            let gidx = table.col_index(gkey).ok_or_else(|| {
                anyhow::anyhow!(
                    "'{gkey}' 열을 찾을 수 없어요. 열: {}",
                    table.headers.join(", ")
                )
            })?;
            let vkey = column
                .ok_or_else(|| anyhow::anyhow!("--그룹 저장은 집계할 --열과 함께 쓰세요."))?;
            let vidx = table.col_index(vkey).ok_or_else(|| {
                anyhow::anyhow!(
                    "'{vkey}' 열을 찾을 수 없어요. 열: {}",
                    table.headers.join(", ")
                )
            })?;
            let headers = vec![
                table.headers[gidx].clone(),
                "합계".into(),
                "평균".into(),
                "건수".into(),
            ];
            // 저장값은 콤마 없는 원시 숫자로(엑셀에서 숫자로 다시 인식되게).
            let raw = |v: f64| {
                if v.fract() == 0.0 {
                    format!("{}", v as i64)
                } else {
                    format!("{v:.2}")
                }
            };
            let rows: Vec<Vec<String>> = table
                .group_by(gidx, vidx)
                .into_iter()
                .map(|g| {
                    let avg = if g.numeric_count > 0 {
                        g.sum / g.numeric_count as f64
                    } else {
                        0.0
                    };
                    vec![g.key, raw(g.sum), raw(avg), g.row_count.to_string()]
                })
                .collect();
            (headers, rows)
        } else {
            let order = match sort {
                Some(skey) => {
                    let sidx = table.col_index(skey).ok_or_else(|| {
                        anyhow::anyhow!(
                            "'{skey}' 열을 찾을 수 없어요. 열: {}",
                            table.headers.join(", ")
                        )
                    })?;
                    table.sorted_rows(sidx, ascending)
                }
                None => (0..table.rows.len()).collect(),
            };
            let rows: Vec<Vec<String>> = order.iter().map(|&i| table.rows[i].clone()).collect();
            (table.headers.clone(), rows)
        };
        let csv = sheet::to_csv(&out_headers, &out_rows);
        util::atomic_write(Path::new(out), csv.as_bytes())
            .map_err(|e| anyhow::anyhow!("저장 실패: {out} ({e})"))?;
        println!();
        println!("  💾 저장: {} ({}행)", out.bright_yellow(), out_rows.len());
        println!();
        return Ok(());
    }

    // 그룹별 집계(피벗): --그룹 지점 --열 매출액 → 지점별 매출 합계·평균.
    if let Some(gkey) = group {
        let gidx = table.col_index(gkey).ok_or_else(|| {
            anyhow::anyhow!(
                "'{gkey}' 열을 찾을 수 없어요. 열: {}",
                table.headers.join(", ")
            )
        })?;
        let vkey = column.ok_or_else(|| {
            anyhow::anyhow!("--그룹은 집계할 --열과 함께 쓰세요. 예: --그룹 {gkey} --열 <숫자열>")
        })?;
        let vidx = table.col_index(vkey).ok_or_else(|| {
            anyhow::anyhow!(
                "'{vkey}' 열을 찾을 수 없어요. 열: {}",
                table.headers.join(", ")
            )
        })?;
        let groups = table.group_by(gidx, vidx);
        println!();
        println!(
            "  📊 '{}'별 '{}' 집계 ({}개 그룹, 합계순)",
            table.headers[gidx],
            table.headers[vidx],
            groups.len()
        );
        println!();
        let namew = groups
            .iter()
            .map(|g| card::disp_width(&g.key))
            .max()
            .unwrap_or(4)
            .clamp(4, 20);
        let maxsum = groups.first().map(|g| g.sum).unwrap_or(0.0);
        let mut total = 0.0;
        for g in &groups {
            total += g.sum;
            let name = card::truncate_pad(&g.key, namew);
            let bar = card::hbar(g.sum, maxsum, 14);
            let avg = if g.numeric_count > 0 {
                g.sum / g.numeric_count as f64
            } else {
                0.0
            };
            println!(
                "     {name}  {}  합계 {}  평균 {}  ({}건)",
                bar.bright_cyan(),
                fmt_stat_num(g.sum).bright_white(),
                fmt_stat_num(avg),
                g.row_count
            );
        }
        println!(
            "     {}",
            format!("─ 전체 합계 {}", fmt_stat_num(total)).dimmed()
        );
        println!();
        return Ok(());
    }

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
            println!("     개수   {}개 (숫자 {}개)", table.rows.len(), nums.len());
            println!("     합계   {}", fmt_stat_num(sum));
            println!("     평균   {}", fmt_stat_num(avg));
            println!("     최대   {}", fmt_stat_num(max));
            println!("     최소   {}", fmt_stat_num(min));
        }
        println!();
        return Ok(());
    }

    // 열 목록 + 미리보기(정렬 옵션이 있으면 그 열 기준으로).
    println!("  열: {}", table.headers.join(" · ").dimmed());
    let order: Vec<usize> = if let Some(skey) = sort {
        let sidx = table.col_index(skey).ok_or_else(|| {
            anyhow::anyhow!(
                "'{skey}' 열을 찾을 수 없어요. 열: {}",
                table.headers.join(", ")
            )
        })?;
        table.sorted_rows(sidx, ascending)
    } else {
        (0..table.rows.len()).collect()
    };
    let n = preview_rows.min(order.len());
    if n > 0 {
        println!();
        match sort {
            Some(skey) => {
                let dir = if ascending {
                    "오름차순"
                } else {
                    "큰 값부터"
                };
                println!("  미리보기 ('{skey}' {dir} 상위 {n}행):");
            }
            None => println!("  미리보기 (상위 {n}행):"),
        }
        for &ri in order.iter().take(n) {
            let cells: Vec<String> = table.rows[ri]
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
    println!(
        "  {} 묶어서 집계(피벗): {}",
        "팁".dimmed(),
        format!("wonjang 엑셀 {file} --그룹 <분류열> --열 <숫자열>").dimmed()
    );
    println!(
        "  {} 정렬해서 상위 행: {}",
        "팁".dimmed(),
        format!("wonjang 엑셀 {file} --정렬 <열이름>").dimmed()
    );
    println!(
        "  {} 조건 행만(조합 가능): {}",
        "팁".dimmed(),
        format!("wonjang 엑셀 {file} --필터 지역=서울").dimmed()
    );
    println!(
        "  {} 결과를 새 CSV로: {}",
        "팁".dimmed(),
        format!("wonjang 엑셀 {file} --필터 지역=서울 --저장 서울만.csv").dimmed()
    );
    println!();
    Ok(())
}

/// 여러 표(CSV·엑셀)를 머리글 기준으로 합쳐 미리보기하거나 새 CSV로 저장한다.
/// GPT가 못 하는 일: 내 컴퓨터의 월별·지점별 파일들을 실제로 읽어 한 장으로.
fn cmd_merge_tables(files: &[String], save: Option<&str>, source: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::path::Path;
    if files.len() < 2 {
        anyhow::bail!(
            "합칠 파일을 2개 이상 적어주세요. 예: wonjang 표합치기 1월.csv 2월.csv --저장 합본.csv"
        );
    }
    // 각 파일을 읽고 라벨(파일 이름, 확장자 제외)을 붙인다.
    let mut loaded: Vec<(String, sheet::Table)> = Vec::new();
    for f in files {
        let t = sheet::Table::load(f).map_err(|e| anyhow::anyhow!("{f}: {e}"))?;
        let label = Path::new(f)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(f)
            .to_string();
        loaded.push((label, t));
    }
    let refs: Vec<(String, &sheet::Table)> = loaded.iter().map(|(l, t)| (l.clone(), t)).collect();
    let (headers, rows) = sheet::merge_tables(&refs, source);

    println!();
    println!(
        "  🧩 표 {}개 합치기 → {}행 × {}열",
        files.len(),
        rows.len(),
        headers.len()
    );
    println!("  열: {}", headers.join(" · ").dimmed());

    if let Some(out) = save {
        if files.iter().any(|f| Path::new(f) == Path::new(out)) {
            anyhow::bail!("입력 파일을 덮어쓸 수 없어요. 다른 경로로 저장하세요.");
        }
        let csv = sheet::to_csv(&headers, &rows);
        util::atomic_write(Path::new(out), csv.as_bytes())
            .map_err(|e| anyhow::anyhow!("저장 실패: {out} ({e})"))?;
        println!();
        println!("  💾 저장: {} ({}행)", out.bright_yellow(), rows.len());
        ui::note("     이제 wonjang 엑셀로 분석할 수 있어요(--그룹·--필터·--정렬).");
    } else {
        let n = 5.min(rows.len());
        if n > 0 {
            println!();
            println!("  미리보기 (상위 {n}행):");
            for row in rows.iter().take(n) {
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
            "  {} 파일로 저장: {}",
            "팁".dimmed(),
            "wonjang 표합치기 <파일들> --저장 합본.csv  (--출처로 출처 열 추가)".dimmed()
        );
    }
    println!();
    Ok(())
}

fn cmd_soul(preset: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    match preset {
        Some("초기화") | Some("기본값") => {
            soul::reset()?;
            println!();
            println!("  🎭 성격을 기본(다정한 비서)으로 되돌렸어요.");
            println!();
        }
        Some(name) => {
            soul::set_preset(name)?;
            println!();
            println!("  🎭 이제부터 '{}' 성격으로 말할게요.", name.bright_cyan());
            println!(
                "     {}",
                soul::active_persona().lines().next().unwrap_or("").dimmed()
            );
            println!();
        }
        None => {
            println!();
            println!("  🎭 원장 성격 (말투·태도)");
            println!(
                "     지금: {}",
                soul::active_persona()
                    .chars()
                    .take(40)
                    .collect::<String>()
                    .dimmed()
            );
            println!();
            println!("  고를 수 있는 성격:");
            for (key, label, _) in soul::PRESETS {
                println!("     {:<6} {}", key.bright_cyan(), label);
            }
            println!();
            println!("     바꾸기: {}", "wonjang 성격 <이름>".bold());
            println!(
                "     직접 편집: {}",
                soul::soul_path()?.display().to_string().dimmed()
            );
            println!("     되돌리기: wonjang 성격 초기화");
            println!();
        }
    }
    Ok(())
}

/// 여러 lopdf Document를 하나로 병합한다(공식 merge 예제 기반, 북마크/아웃라인 제외).
/// 반환: (합쳐진 Document, 총 페이지 수). 파일 IO가 없어 단위 테스트로 검증 가능.
fn merge_documents(docs: Vec<lopdf::Document>) -> Result<(lopdf::Document, usize)> {
    use lopdf::{Document, Object, ObjectId};
    use std::collections::BTreeMap;

    let mut max_id = 1;
    let mut documents_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut documents_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut document = Document::with_version("1.5");

    for mut doc in docs {
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;
        documents_pages.extend(
            doc.get_pages()
                .into_values()
                .map(|object_id| (object_id, doc.get_object(object_id).unwrap().to_owned())),
        );
        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.into_iter() {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        object_id
                    },
                    object,
                ));
            }
            b"Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }
                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            b"Page" => {}
            b"Outlines" => {}
            b"Outline" => {}
            _ => {
                document.objects.insert(object_id, object);
            }
        }
    }

    let pages_object =
        pages_object.ok_or_else(|| anyhow::anyhow!("Pages 루트를 찾지 못했어요(손상된 PDF?)."))?;
    let catalog_object = catalog_object
        .ok_or_else(|| anyhow::anyhow!("Catalog 루트를 찾지 못했어요(손상된 PDF?)."))?;

    // 모든 페이지의 부모를 새 Pages로.
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.0);
            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    let (catalog_id, catalog_object) = catalog_object;
    let (page_id, page_object) = pages_object;
    let page_count = documents_pages.len();

    if let Ok(dictionary) = page_object.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", page_count as u32);
        dictionary.set(
            "Kids",
            documents_pages
                .into_keys()
                .map(Object::Reference)
                .collect::<Vec<_>>(),
        );
        document
            .objects
            .insert(page_id, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_object.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", page_id);
        dictionary.remove(b"Outlines");
        document
            .objects
            .insert(catalog_id, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_id);
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    Ok((document, page_count))
}

/// 여러 PDF 파일을 하나로 합친다(서류 합본 제출 — GPT가 못 만지는 내 로컬 PDF).
fn cmd_pdf_merge(files: &[String], output: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::path::{Path, PathBuf};
    if files.len() < 2 {
        anyhow::bail!("합칠 PDF를 2개 이상 주세요. 예: wonjang pdf합치기 1.pdf 2.pdf");
    }
    let mut docs = Vec::new();
    for f in files {
        if !f.to_lowercase().ends_with(".pdf") {
            anyhow::bail!("PDF 파일이 아니에요: {f}");
        }
        if !Path::new(f).exists() {
            anyhow::bail!("파일을 찾을 수 없어요: {f}");
        }
        let doc = lopdf::Document::load(f).map_err(|e| {
            anyhow::anyhow!("PDF를 열지 못했어요({f}): {e}. 암호·손상 여부를 확인해 주세요.")
        })?;
        if doc.is_encrypted() {
            anyhow::bail!("비밀번호가 걸린 PDF예요: {f}. 먼저 PDF 뷰어에서 암호를 풀어 저장한 뒤 다시 시도하세요.");
        }
        docs.push(doc);
    }
    let (mut merged, count) = merge_documents(docs)?;

    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => PathBuf::from("합본.pdf"),
    };
    if files.iter().any(|f| Path::new(f) == out_path) {
        anyhow::bail!("출력 경로가 입력 PDF 중 하나와 같아요. 다른 경로(--출력)를 쓰세요.");
    }
    merged.save(&out_path)?;
    let bytes = std::fs::metadata(&out_path)?.len();
    println!();
    println!(
        "  📎 PDF {}개 → {}페이지로 합침",
        files.len(),
        count.to_string().bright_white()
    );
    println!(
        "     저장  {}  ({})",
        out_path.display().to_string().bright_yellow(),
        human_bytes(bytes)
    );
    println!();
    Ok(())
}

/// 회전 각도를 검증·정규화한다(90의 배수, 0~270 범위). 순수.
fn normalize_rotation(angle: i64) -> Result<i64> {
    if angle % 90 != 0 {
        anyhow::bail!("회전 각도는 90의 배수여야 해요(90, 180, 270, -90).");
    }
    Ok(angle.rem_euclid(360))
}

/// PDF에 비밀번호(AES-256)를 걸어 새 PDF로 저장한다(민감 서류 보호). 원본 보존.
fn cmd_pdf_encrypt(file: &str, password: &str, output: Option<&str>) -> Result<()> {
    use lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
    use lopdf::{EncryptionState, EncryptionVersion, Permissions};
    use owo_colors::OwoColorize;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    if password.is_empty() {
        anyhow::bail!("비밀번호를 입력해 주세요(--비번 <비밀번호>).");
    }
    let path = Path::new(file);
    if !path.exists() {
        anyhow::bail!("파일을 찾을 수 없어요: {file}");
    }
    if !file.to_lowercase().ends_with(".pdf") {
        anyhow::bail!("PDF 파일이 아니에요: {file}");
    }
    let mut doc = lopdf::Document::load(path).map_err(|e| {
        anyhow::anyhow!("PDF를 열지 못했어요({e}). 손상됐거나 PDF가 아닐 수 있어요.")
    })?;
    if doc.is_encrypted() {
        anyhow::bail!("이미 비밀번호가 걸린 PDF예요.");
    }

    // AES-256(V5)로 암호화 — 열 때 비밀번호가 필요하도록 user/owner 모두 설정.
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).map_err(|e| anyhow::anyhow!("난수 생성 실패: {e}"))?;
    let crypt_filter: Arc<dyn CryptFilter> = Arc::new(Aes256CryptFilter);
    let version = EncryptionVersion::V5 {
        encrypt_metadata: true,
        crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), crypt_filter)]),
        file_encryption_key: &key,
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: password,
        user_password: password,
        permissions: Permissions::all(),
    };
    let state =
        EncryptionState::try_from(version).map_err(|e| anyhow::anyhow!("암호화 설정 실패: {e}"))?;
    doc.encrypt(&state)
        .map_err(|e| anyhow::anyhow!("암호화 실패: {e}"))?;

    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("pdf");
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(format!("{stem}_암호.pdf"))
        }
    };
    if out_path == path {
        anyhow::bail!("출력 경로가 원본과 같아요. 원본 보존을 위해 다른 경로(--출력)를 쓰세요.");
    }
    doc.save(&out_path)?;
    println!();
    println!("  🔒 PDF에 비밀번호를 걸었어요 (AES-256)");
    println!(
        "     저장  {}",
        out_path.display().to_string().bright_yellow()
    );
    ui::note("     ⚠ 이 비밀번호를 잊으면 파일을 열 수 없어요. 안전하게 보관하세요.");
    println!();
    Ok(())
}

/// PDF 페이지에 회전(/Rotate)을 적용해 새 PDF로 저장한다(옆으로 스캔된 서류 바로 세우기).
fn cmd_pdf_rotate(file: &str, angle: i64, pages: Option<&str>, output: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    let path = Path::new(file);
    if !path.exists() {
        anyhow::bail!("파일을 찾을 수 없어요: {file}");
    }
    if !file.to_lowercase().ends_with(".pdf") {
        anyhow::bail!("PDF 파일이 아니에요: {file}");
    }
    let norm = normalize_rotation(angle)?;
    if norm == 0 {
        anyhow::bail!("회전 각도가 0이에요(90·180·270을 주세요).");
    }
    let mut doc = lopdf::Document::load(path).map_err(|e| {
        anyhow::anyhow!("PDF를 열지 못했어요({e}). 암호가 걸렸거나 손상됐을 수 있어요.")
    })?;
    if doc.is_encrypted() {
        anyhow::bail!(
            "비밀번호가 걸린 PDF예요. 먼저 PDF 뷰어에서 암호를 풀어 저장한 뒤 다시 시도하세요."
        );
    }
    let page_map = doc.get_pages();
    let total = page_map.len() as u32;
    if total == 0 {
        anyhow::bail!("페이지가 없는 PDF예요.");
    }
    let targets: BTreeSet<u32> = match pages {
        Some(spec) => parse_page_range(spec, total)?.into_iter().collect(),
        None => (1..=total).collect(),
    };

    let mut rotated = 0usize;
    for (num, page_id) in page_map {
        if !targets.contains(&num) {
            continue;
        }
        if let Ok(dict) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
            let current = dict.get(b"Rotate").and_then(|o| o.as_i64()).unwrap_or(0);
            // 손상/악성 PDF의 비정상 /Rotate(i64::MAX 등)와 더하면 오버플로 → 먼저 정규화.
            dict.set("Rotate", (current.rem_euclid(360) + norm) % 360);
            rotated += 1;
        }
    }

    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("pdf");
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(format!("{stem}_회전.pdf"))
        }
    };
    if out_path == path {
        anyhow::bail!("출력 경로가 원본과 같아요. 다른 경로(--출력)를 쓰세요.");
    }
    doc.save(&out_path)?;
    println!();
    println!(
        "  🔄 PDF {}페이지 {}도 회전",
        rotated.to_string().bright_white(),
        norm
    );
    println!(
        "     저장  {}",
        out_path.display().to_string().bright_yellow()
    );
    println!();
    Ok(())
}

/// "1-3,5,8-10" 같은 범위를 1-based 페이지 번호(정렬·중복제거)로. total 초과·0은 오류. 순수.
fn parse_page_range(spec: &str, total: u32) -> Result<Vec<u32>> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<u32> = BTreeSet::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            let a: u32 = a
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("범위를 이해 못했어요: '{part}'"))?;
            let b: u32 = b
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("범위를 이해 못했어요: '{part}'"))?;
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            // 펼치기 전에 상한 검증(큰 범위로 메모리 폭주/멈춤 방지).
            if hi > total {
                anyhow::bail!("이 PDF는 {total}페이지인데 {hi}페이지를 요청했어요.");
            }
            for p in lo..=hi {
                set.insert(p);
            }
        } else {
            let p: u32 = part
                .parse()
                .map_err(|_| anyhow::anyhow!("페이지 번호를 이해 못했어요: '{part}'"))?;
            if p > total {
                anyhow::bail!("이 PDF는 {total}페이지인데 {p}페이지를 요청했어요.");
            }
            set.insert(p);
        }
    }
    if set.contains(&0) {
        anyhow::bail!("페이지는 1부터 시작해요(0은 없어요).");
    }
    if set.is_empty() {
        anyhow::bail!("추출할 페이지를 지정해주세요. 예: 1-3,5");
    }
    Ok(set.into_iter().collect())
}

/// PDF에서 지정한 페이지만 남겨 새 PDF로 저장한다(라이브러리 delete_pages 사용·원본 보존).
fn cmd_pdf_pages(file: &str, range: &str, output: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    let path = Path::new(file);
    if !path.exists() {
        anyhow::bail!("파일을 찾을 수 없어요: {file}");
    }
    if !file.to_lowercase().ends_with(".pdf") {
        anyhow::bail!("PDF 파일이 아니에요: {file}");
    }
    let mut doc = lopdf::Document::load(path).map_err(|e| {
        anyhow::anyhow!("PDF를 열지 못했어요({e}). 암호가 걸렸거나 손상됐을 수 있어요.")
    })?;
    if doc.is_encrypted() {
        anyhow::bail!(
            "비밀번호가 걸린 PDF예요. 먼저 PDF 뷰어에서 암호를 풀어 저장한 뒤 다시 시도하세요."
        );
    }
    let total = doc.get_pages().len() as u32;
    if total == 0 {
        anyhow::bail!("페이지가 없는 PDF예요.");
    }
    let keep = parse_page_range(range, total)?;
    let keep_set: BTreeSet<u32> = keep.iter().copied().collect();
    let to_delete: Vec<u32> = (1..=total).filter(|p| !keep_set.contains(p)).collect();
    if to_delete.len() as u32 == total {
        anyhow::bail!("남길 페이지가 없어요.");
    }
    doc.delete_pages(&to_delete);
    doc.prune_objects();

    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("pdf");
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(format!("{stem}_페이지.pdf"))
        }
    };
    if out_path == path {
        anyhow::bail!("출력 경로가 원본과 같아요. 원본 보존을 위해 다른 경로(--출력)를 쓰세요.");
    }
    doc.save(&out_path)?;
    let new_total = doc.get_pages().len();
    let bytes = std::fs::metadata(&out_path)?.len();
    println!();
    println!(
        "  ✂️  PDF {total}페이지 중 {}페이지 추출",
        new_total.to_string().bright_white()
    );
    println!(
        "     페이지  {}",
        keep.iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
            .dimmed()
    );
    println!(
        "     저장    {}  ({})",
        out_path.display().to_string().bright_yellow(),
        human_bytes(bytes)
    );
    println!();
    Ok(())
}

/// EXIF 방향을 적용해 이미지를 연다(폰 사진이 옆으로 눕지 않도록). 방향을 못 읽으면 일반 open 폴백.
fn open_image_oriented(path: &std::path::Path) -> Result<image::DynamicImage> {
    use image::ImageDecoder;
    let oriented = (|| -> image::ImageResult<image::DynamicImage> {
        let mut decoder = image::ImageReader::open(path)?
            .with_guessed_format()?
            .into_decoder()?;
        let orientation = decoder.orientation()?;
        let mut img = image::DynamicImage::from_decoder(decoder)?;
        img.apply_orientation(orientation);
        Ok(img)
    })();
    match oriented {
        Ok(img) => Ok(img),
        Err(_) => image::open(path).map_err(|e| anyhow::anyhow!("이미지를 열지 못했어요: {e}")),
    }
}

/// 이미지를 RGB로 바꾸되, 투명한 부분은 **흰 배경**에 합성한다(알파를 그냥 버리면 검게 됨).
/// JPEG로 저장하거나 PDF에 임베드할 때 사용 — 투명 PNG가 검은 배경으로 나오는 문제를 막는다.
fn flatten_on_white(img: &image::DynamicImage) -> image::RgbImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for (x, y, p) in rgba.enumerate_pixels() {
        let [r, g, b, a] = p.0;
        if a == 255 {
            out.put_pixel(x, y, image::Rgb([r, g, b]));
        } else {
            let af = a as f32 / 255.0;
            let blend = |c: u8| {
                ((c as f32 * af) + 255.0 * (1.0 - af))
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            out.put_pixel(x, y, image::Rgb([blend(r), blend(g), blend(b)]));
        }
    }
    out
}

/// 확장자가 우리가 다루는 이미지(JPEG/PNG)인지. 순수 함수.
fn is_supported_image_ext(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
}

/// 여러 이미지를 한 PDF로 묶는다(각 이미지가 한 페이지). JPEG는 DCTDecode로 임베드.
/// 반환: 만든 페이지 수. (GPT가 못 만지는 내 로컬 사진 → 제출용 PDF)
fn cmd_photos_pdf(files: &[String], output: Option<&str>) -> Result<()> {
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Document, Object, Stream};
    use owo_colors::OwoColorize;
    use std::path::{Path, PathBuf};

    // 입력 검증.
    for f in files {
        if !is_supported_image_ext(f) {
            anyhow::bail!("JPEG·PNG만 묶을 수 있어요(문제 파일: {f}). HEIC는 미지원이에요.");
        }
        if !Path::new(f).exists() {
            anyhow::bail!("파일을 찾을 수 없어요: {f}");
        }
    }

    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => PathBuf::from("묶음.pdf"),
    };

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids: Vec<Object> = Vec::new();

    for f in files {
        // EXIF 방향 적용해 열고(폰 사진 정방향), RGB 베이스라인 JPEG로 임베드.
        // 투명 PNG는 흰 배경에 합성(검게 임베드되지 않도록).
        let img = open_image_oriented(Path::new(f))?;
        let rgb = flatten_on_white(&img);
        let (w, h) = (rgb.width(), rgb.height());
        let mut jpeg: Vec<u8> = Vec::new();
        {
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85);
            enc.encode_image(&rgb)?;
        }

        let img_stream = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => w as i64,
                "Height" => h as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            jpeg,
        );
        let img_id = doc.add_object(img_stream);

        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Im0" => img_id },
        });

        // 이미지를 페이지(=이미지 픽셀 크기) 전체에 그린다.
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        (w as i64).into(),
                        0.into(),
                        0.into(),
                        (h as i64).into(),
                        0.into(),
                        0.into(),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(b"Im0".to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), (w as i64).into(), (h as i64).into()],
            "Contents" => content_id,
            "Resources" => resources_id,
        });
        kids.push(page_id.into());
    }

    let count = kids.len();
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => kids,
        "Count" => count as i64,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.save(&out_path)?;

    let bytes = std::fs::metadata(&out_path)?.len();
    println!();
    println!(
        "  📚 사진 {}장 → PDF {}페이지",
        files.len(),
        count.to_string().bright_white()
    );
    println!(
        "     저장  {}  ({})",
        out_path.display().to_string().bright_yellow(),
        human_bytes(bytes)
    );
    println!();
    Ok(())
}

/// 여러 이미지를 한 장으로 이어붙인다(세로 쌓기 기본, --가로면 나란히). EXIF 방향 적용.
/// GPT가 못 하는 일: 내 컴퓨터의 캡처·영수증 사진들을 실제로 합쳐 한 장으로.
fn cmd_image_stitch(files: &[String], horizontal: bool, output: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::path::{Path, PathBuf};

    if files.len() < 2 {
        anyhow::bail!(
            "이어붙일 이미지를 2개 이상 적어주세요. 예: wonjang 이미지이어붙이기 1.png 2.png --세로"
        );
    }
    // 로드(EXIF 방향 적용 — 옆으로 누운 폰 사진도 바로).
    let mut imgs: Vec<image::RgbaImage> = Vec::new();
    for f in files {
        if !Path::new(f).exists() {
            anyhow::bail!("파일을 찾을 수 없어요: {f}");
        }
        imgs.push(open_image_oriented(Path::new(f))?.to_rgba8());
    }
    // 캔버스 크기: 세로면 폭=최대, 높이=합 / 가로면 높이=최대, 폭=합.
    let (cw, ch) = if horizontal {
        (
            imgs.iter().map(|i| i.width()).sum::<u32>(),
            imgs.iter().map(|i| i.height()).max().unwrap_or(0),
        )
    } else {
        (
            imgs.iter().map(|i| i.width()).max().unwrap_or(0),
            imgs.iter().map(|i| i.height()).sum::<u32>(),
        )
    };
    if cw == 0 || ch == 0 {
        anyhow::bail!("이미지 크기가 0이에요.");
    }
    // OOM 방지: 합친 캔버스가 지나치게 크면 거부하고 줄이기를 안내.
    if cw as u64 * ch as u64 > 100_000_000 {
        anyhow::bail!(
            "합치면 너무 커요({cw}×{ch}). 먼저 'wonjang 이미지 <파일> --폭 1280'으로 줄여보세요."
        );
    }
    // 흰 배경 캔버스에 차례로 합성(투명 영역은 흰색으로).
    let mut canvas = image::RgbaImage::from_pixel(cw, ch, image::Rgba([255, 255, 255, 255]));
    let (mut x, mut y) = (0i64, 0i64);
    for img in &imgs {
        image::imageops::overlay(&mut canvas, img, x, y);
        if horizontal {
            x += img.width() as i64;
        } else {
            y += img.height() as i64;
        }
    }
    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => PathBuf::from("이어붙임.png"),
    };
    if files.iter().any(|f| Path::new(f) == out_path) {
        anyhow::bail!("원본을 덮어쓸 수 없어요. 다른 출력 경로를 쓰세요(--출력).");
    }
    // JPEG 등도 되도록 RGB로 변환해 저장(흰 배경이라 알파 불필요).
    let rgb = image::DynamicImage::ImageRgba8(canvas).to_rgb8();
    rgb.save(&out_path)
        .map_err(|e| anyhow::anyhow!("저장 실패: {e}"))?;
    let bytes = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    println!();
    println!(
        "  🧵 이미지 {}장을 {}로 이어붙였어요",
        files.len(),
        if horizontal { "가로" } else { "세로" }
    );
    println!(
        "     저장  {}  ({}×{}, {})",
        out_path.display().to_string().bright_yellow(),
        cw,
        ch,
        human_bytes(bytes)
    );
    println!();
    Ok(())
}

/// CP949(EUC-KR 슈퍼셋) 바이트를 UTF-8 문자열로 디코드한다(text, 깨진 글자 있었는지). 순수.
fn cp949_to_utf8(bytes: &[u8]) -> (String, bool) {
    let (cow, _, had_errors) = encoding_rs::EUC_KR.decode(bytes);
    (cow.into_owned(), had_errors)
}

/// UTF-8 문자열을 CP949 바이트로 인코드한다(bytes, 표현 못한 글자 있었는지). 순수.
fn utf8_to_cp949(text: &str) -> (Vec<u8>, bool) {
    let (cow, _, had_errors) = encoding_rs::EUC_KR.encode(text);
    (cow.into_owned(), had_errors)
}

/// 디코드된 텍스트가 "한글이 정상으로 보이는지" 대략 판정(치환문자·제어문자 비율). 순수.
fn looks_like_korean_text(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut bad = 0usize;
    let mut total = 0usize;
    for c in s.chars() {
        total += 1;
        // U+FFFD(치환) 또는 흔치 않은 제어문자는 깨짐 신호.
        if c == '\u{FFFD}' || (c.is_control() && !matches!(c, '\n' | '\r' | '\t')) {
            bad += 1;
        }
    }
    (bad as f64 / total as f64) < 0.02
}

/// 한글 깨진 파일(CP949/EUC-KR)을 UTF-8로 복구한다(GPT가 못 만지는 내 로컬 파일 바이트).
fn cmd_encfix(file: &str, output: Option<&str>, reverse: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::path::{Path, PathBuf};
    let path = Path::new(file);
    if !path.exists() {
        anyhow::bail!("파일을 찾을 수 없어요: {file}");
    }
    let bytes = std::fs::read(path)?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    if reverse {
        // UTF-8 → CP949(옛 한국 시스템 업로드용).
        let text = String::from_utf8(bytes).map_err(|_| {
            anyhow::anyhow!("이 파일은 UTF-8이 아니에요. 되돌리기는 UTF-8 파일에만 써요.")
        })?;
        let (out, lossy) = utf8_to_cp949(&text);
        let out_path = match output {
            Some(o) => PathBuf::from(o),
            None => parent.join(format!("{stem}_cp949.{ext}")),
        };
        std::fs::write(&out_path, &out)?;
        println!();
        println!("  🔤 {} → CP949(EUC-KR)", file.bright_cyan());
        if lossy {
            ui::note("     ⚠ CP949로 표현 못 하는 글자가 있어 일부 손실됐어요(이모지 등).");
        }
        println!(
            "     저장  {}",
            out_path.display().to_string().bright_yellow()
        );
        println!();
        return Ok(());
    }

    // 기본: 깨진 한글(CP949) → UTF-8 복구.
    if std::str::from_utf8(&bytes).is_ok() {
        ui::note("이미 UTF-8이에요 — 복구할 게 없어요. (옛 시스템용 변환은 --되돌리기)");
        return Ok(());
    }
    let (text, had_errors) = cp949_to_utf8(&bytes);
    if had_errors && !looks_like_korean_text(&text) {
        anyhow::bail!(
            "CP949로도 정상 복구가 안 돼요. 다른 인코딩이거나 텍스트 파일이 아닐 수 있어요."
        );
    }
    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => parent.join(format!("{stem}_utf8.{ext}")),
    };
    std::fs::write(&out_path, text.as_bytes())?;
    println!();
    println!("  🔤 {} (CP949/EUC-KR) → UTF-8 복구", file.bright_cyan());
    // 복구된 앞부분 미리보기(깨짐→정상 확인).
    let preview: Vec<&str> = text.lines().take(3).collect();
    if !preview.is_empty() {
        println!();
        for l in preview {
            let shown: String = l.chars().take(60).collect();
            println!("     {}", shown.dimmed());
        }
    }
    println!();
    println!(
        "     저장  {}",
        out_path.display().to_string().bright_yellow()
    );
    println!();
    Ok(())
}

/// 원본 크기와 옵션(목표 폭/배율)으로 결과 크기를 계산한다(비율 유지·축소 전용). 순수 함수.
/// 확대는 하지 않는다(첨부 용량 줄이기 목적) — 조건이 맞지 않으면 원본 크기 그대로.
fn plan_resize(w: u32, h: u32, target_w: Option<u32>, scale: Option<f64>) -> (u32, u32) {
    if let Some(tw) = target_w {
        if tw == 0 || w == 0 || tw >= w {
            return (w, h);
        }
        let nh = ((h as f64) * (tw as f64) / (w as f64)).round() as u32;
        return (tw, nh.max(1));
    }
    if let Some(s) = scale {
        // NaN·무한대(잘못된 --배율 입력)는 안전하게 원본 유지.
        if !s.is_finite() || s <= 0.0 || s >= 1.0 {
            return (w, h);
        }
        let nw = ((w as f64) * s).round() as u32;
        let nh = ((h as f64) * s).round() as u32;
        return (nw.max(1), nh.max(1));
    }
    (w, h)
}

/// 사람이 읽기 좋은 바이트 표기.
fn human_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1}MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.0}KB", n as f64 / 1024.0)
    } else {
        format!("{n}B")
    }
}

/// 로컬 이미지를 축소·압축한다(GPT가 못 만지는 내 사진 — 첨부 용량 줄이기). 원본은 보존.
/// 받은편지함을 읽는다(IMAP). 미설정 시 친절한 설정 안내를 보여준다.
fn cmd_mail(count: usize, unseen: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    let cfg = match email::EmailConfig::from_env() {
        Some(c) => c,
        None => {
            print_mail_setup_help();
            return Ok(());
        }
    };
    ui::info(&format!(
        "📬 {} 받은편지함을 확인하는 중… ({}:{})",
        cfg.user, cfg.host, cfg.port
    ));
    let view = email::fetch_inbox(&cfg, count.max(1), unseen)?;
    println!();
    println!(
        "  📬 {}  (전체 {}통 · 안읽음 {}통)",
        cfg.user.bright_cyan(),
        view.total,
        view.unseen.to_string().bright_yellow()
    );
    if view.headers.is_empty() {
        ui::info(if unseen {
            "     안 읽은 메일이 없어요. 깔끔하네요 ✨"
        } else {
            "     받은 메일이 없어요."
        });
        println!();
        return Ok(());
    }
    println!();
    for (i, h) in view.headers.iter().enumerate() {
        let mark = if h.unseen {
            "●".bright_yellow().to_string()
        } else {
            "·".dimmed().to_string()
        };
        let subject = if h.subject.chars().count() > 50 {
            format!("{}…", h.subject.chars().take(50).collect::<String>())
        } else {
            h.subject.clone()
        };
        let subject = if h.unseen {
            subject.bright_white().bold().to_string()
        } else {
            subject.clone()
        };
        // 표시 순번(1=최신) — `메일읽기 N`으로 바로 열 수 있게.
        println!("  {:>2}. {mark} {subject}", (i + 1).to_string().dimmed());
        println!(
            "       {}  {}",
            h.from.dimmed(),
            short_mail_date(&h.date).dimmed()
        );
    }
    println!();
    ui::info("     (본문 읽기: wonjang 메일읽기 <번호> · 안 읽은 것만: --안읽음)");
    println!();
    Ok(())
}

/// 메일을 보낸다(SMTP, 첨부 가능). 외부 전송이므로 보내기 전 받는사람·제목·첨부를 명확히 echo한다.
fn cmd_mail_send(to: &str, subject: &str, body: &str, attach: &[String]) -> Result<()> {
    use anyhow::Context;
    use owo_colors::OwoColorize;
    use std::path::Path;
    let cfg = match email::EmailConfig::from_env() {
        Some(c) => c,
        None => {
            print_mail_setup_help();
            return Ok(());
        }
    };
    // 받는 사람을 실제 파싱으로 미리 검증(첨부 읽기·전송 박스 출력 전에 명확히 거부).
    email::validate_recipient(to)?;
    // 첨부 파일 읽기(보내기 전에 확인).
    let mut attachments: Vec<(String, Vec<u8>)> = Vec::new();
    for p in attach {
        let path = Path::new(p);
        if !path.exists() {
            anyhow::bail!("첨부할 파일을 찾을 수 없어요: {p}");
        }
        let bytes = std::fs::read(path).with_context(|| format!("첨부 읽기 실패: {p}"))?;
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment")
            .to_string();
        attachments.push((filename, bytes));
    }
    // 외부 전송 — 무엇을 보내는지 먼저 명확히 보여준다.
    println!();
    println!("  📤 {}", "메일을 보냅니다".bright_cyan().bold());
    println!("     {} {}", "보내는이".dimmed(), cfg.user);
    println!("     {} {}", "받는이  ".dimmed(), to.bright_white());
    println!("     {} {}", "제목    ".dimmed(), subject);
    let preview: String = body.chars().take(80).collect();
    println!(
        "     {} {}{}",
        "내용    ".dimmed(),
        preview,
        if body.chars().count() > 80 { "…" } else { "" }
    );
    for (name, bytes) in &attachments {
        println!(
            "     {} {}  ({})",
            "첨부    ".dimmed(),
            name.bright_white(),
            human_bytes(bytes.len() as u64)
        );
    }
    println!();
    ui::info(&format!("     SMTP {} 로 전송 중…", cfg.smtp_host));
    email::send_mail(&cfg, to, subject, body, &attachments)?;
    println!();
    println!(
        "  ✅ {} 에게 보냈어요!{}",
        to.bright_white(),
        if attachments.is_empty() {
            String::new()
        } else {
            format!(" (첨부 {}개)", attachments.len())
        }
    );
    println!();
    Ok(())
}

/// 받은편지함에서 보낸이·제목으로 메일을 찾는다(최근 N통을 받아 클라이언트 측 필터 — 한글 안전).
fn cmd_mail_search(query: &str, scan: usize) -> Result<()> {
    use owo_colors::OwoColorize;
    let cfg = match email::EmailConfig::from_env() {
        Some(c) => c,
        None => {
            print_mail_setup_help();
            return Ok(());
        }
    };
    if query.trim().is_empty() {
        anyhow::bail!("검색어를 입력해 주세요. 예: wonjang 메일검색 영수증");
    }
    ui::info(&format!(
        "🔎 최근 {}통에서 '{}' 찾는 중…",
        scan.max(1),
        query
    ));
    let view = email::fetch_inbox(&cfg, scan.max(1), false)?;
    // 원래 목록상 위치(1=최신)를 함께 보존 — 그래야 `메일읽기 <번호>`가 정확히 그 메일을 연다.
    let hits: Vec<(usize, &email::MailHeader)> = view
        .headers
        .iter()
        .enumerate()
        .filter(|(_, h)| email::matches_query(&h.from, &h.subject, query))
        .collect();
    println!();
    println!(
        "  🔎 '{}' — {}통 찾음 (최근 {}통 중)",
        query.bright_white(),
        hits.len().to_string().bright_yellow(),
        view.headers.len()
    );
    if hits.is_empty() {
        ui::info("     일치하는 메일이 없어요. 검색 범위를 넓혀보세요(--최근 300).");
        println!();
        return Ok(());
    }
    println!();
    for (idx, h) in &hits {
        let mark = if h.unseen {
            "●".bright_yellow().to_string()
        } else {
            "·".dimmed().to_string()
        };
        let subject = if h.subject.chars().count() > 50 {
            format!("{}…", h.subject.chars().take(50).collect::<String>())
        } else {
            h.subject.clone()
        };
        // 번호 = 받은편지함 최신순 위치(메일읽기와 동일).
        println!("  {:>2}. {mark} {subject}", (idx + 1).to_string().dimmed());
        println!(
            "       {}  {}",
            h.from.dimmed(),
            short_mail_date(&h.date).dimmed()
        );
    }
    println!();
    ui::info("     (본문 읽기: wonjang 메일읽기 <번호> — 위 번호 그대로)");
    println!();
    Ok(())
}

/// 메일의 첨부파일을 폴더에 저장한다(파일명 충돌 시 (1),(2)… , 경로 탈출 방지).
fn cmd_mail_attach(num: usize, dir: Option<&str>, unseen: bool) -> Result<()> {
    use anyhow::Context;
    use owo_colors::OwoColorize;
    use std::path::{Path, PathBuf};
    let cfg = match email::EmailConfig::from_env() {
        Some(c) => c,
        None => {
            print_mail_setup_help();
            return Ok(());
        }
    };
    ui::info(&format!("📎 {}번째 메일의 첨부를 확인하는 중…", num.max(1)));
    let (subject, atts) = match email::fetch_attachments(&cfg, num, unseen)? {
        Some(v) => v,
        None => {
            ui::note(&format!(
                "{}번째 메일이 없어요. `wonjang 메일`로 확인해 보세요.",
                num.max(1)
            ));
            return Ok(());
        }
    };
    println!();
    println!("  📎 {}", subject.bright_white().bold());
    if atts.is_empty() {
        ui::info("     첨부파일이 없는 메일이에요.");
        println!();
        return Ok(());
    }
    let base = PathBuf::from(dir.unwrap_or("."));
    if !base.exists() {
        std::fs::create_dir_all(&base)
            .with_context(|| format!("폴더를 만들지 못했어요: {}", base.display()))?;
    }
    let mut saved = 0usize;
    for att in &atts {
        // 파일명은 이미 email::safe_filename으로 경로 요소 제거됨.
        let mut target = base.join(&att.filename);
        // 충돌 시 이름 뒤에 (n) 붙이기(덮어쓰기 방지).
        if target.exists() {
            let stem = Path::new(&att.filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = Path::new(&att.filename)
                .extension()
                .and_then(|s| s.to_str())
                .map(|e| format!(".{e}"))
                .unwrap_or_default();
            for n in 1..1000 {
                let cand = base.join(format!("{stem} ({n}){ext}"));
                if !cand.exists() {
                    target = cand;
                    break;
                }
            }
        }
        std::fs::write(&target, &att.bytes)
            .with_context(|| format!("저장 실패: {}", target.display()))?;
        saved += 1;
        println!(
            "     💾 {}  ({})",
            target.display().to_string().bright_yellow(),
            human_bytes(att.bytes.len() as u64)
        );
    }
    println!();
    println!("  ✅ 첨부 {}개 저장 완료", saved.to_string().bright_white());
    println!();
    Ok(())
}

/// 특정 메일의 본문을 읽어 보여준다.
fn cmd_mail_read(num: usize, unseen: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    let cfg = match email::EmailConfig::from_env() {
        Some(c) => c,
        None => {
            print_mail_setup_help();
            return Ok(());
        }
    };
    ui::info(&format!("📖 {}번째 메일을 여는 중…", num.max(1)));
    let mail = match email::fetch_message(&cfg, num, unseen)? {
        Some(m) => m,
        None => {
            ui::note(&format!(
                "{}번째 메일이 없어요. 먼저 `wonjang 메일`로 목록을 확인해 보세요.",
                num.max(1)
            ));
            return Ok(());
        }
    };
    println!();
    println!("  ✉️  {}", mail.subject.bright_white().bold());
    println!("     {} {}", "보낸이".dimmed(), mail.from);
    println!("     {} {}", "날짜  ".dimmed(), short_mail_date(&mail.date));
    println!();
    // 본문(너무 길면 자르기).
    let body = mail.body.trim();
    if body.is_empty() {
        ui::info("     (본문 텍스트가 없어요 — 첨부나 이미지 메일일 수 있어요.)");
    } else {
        let limit = 3000;
        let shown: String = body.chars().take(limit).collect();
        for line in shown.lines() {
            println!("  {line}");
        }
        if body.chars().count() > limit {
            println!();
            ui::info("     … (본문이 길어 앞부분만 보여드렸어요.)");
        }
    }
    println!();
    Ok(())
}

/// 메일 헤더의 Date를 짧게(앞부분만). 파싱 실패 시 원문 일부.
fn short_mail_date(raw: &str) -> String {
    let t = raw.trim();
    // RFC2822: "Wed, 03 Jun 2026 09:15:00 +0900" → "03 Jun 2026 09:15"
    let parts: Vec<&str> = t.split_whitespace().collect();
    if parts.len() >= 5 && parts[0].ends_with(',') {
        let hm = parts[4].split(':').take(2).collect::<Vec<_>>().join(":");
        format!("{} {} {} {}", parts[1], parts[2], parts[3], hm)
    } else {
        t.chars().take(25).collect()
    }
}

/// 이메일 미설정 시 환경변수 설정 안내(앱 비밀번호 강조).
fn print_mail_setup_help() {
    use owo_colors::OwoColorize;
    println!();
    println!(
        "  📭 {}",
        "이메일이 아직 연결되지 않았어요".bright_cyan().bold()
    );
    println!();
    println!("  환경변수 두 개만 설정하면 받은편지함을 읽어드려요:");
    println!("     {}=you@gmail.com", "WONJANG_EMAIL".bright_white());
    println!(
        "     {}=앱비밀번호",
        "WONJANG_EMAIL_PASSWORD".bright_white()
    );
    println!();
    println!(
        "  {}",
        "💡 일반 로그인 비밀번호가 아니라 '앱 비밀번호'가 필요해요(2단계 인증 계정).".yellow()
    );
    println!("     · Gmail: 계정 보안 → 2단계 인증 → 앱 비밀번호 발급");
    println!("     · 네이버: 메일 환경설정 → POP3/IMAP → IMAP 사용 + 앱 비밀번호");
    println!("     · 호스트는 도메인으로 자동 추정(필요 시 WONJANG_EMAIL_HOST/PORT로 지정)");
    println!();
    println!(
        "  {}",
        "설정 후: wonjang 메일  ·  안 읽은 것만: wonjang 메일 --안읽음".dimmed()
    );
    println!();
}

/// 이미지 여러 장을 일괄로 줄인다(중고거래·블로그·메일에 사진 여러 장 올리기). 원본 보존.
fn cmd_image(
    files: &[String],
    width: Option<u32>,
    scale: Option<f64>,
    quality: u8,
    format: Option<&str>,
    output: Option<&str>,
) -> Result<()> {
    if output.is_some() && files.len() > 1 {
        anyhow::bail!(
            "여러 장엔 --출력을 쓸 수 없어요(각 파일 옆에 _작게로 저장돼요). 한 장만 지정하세요."
        );
    }
    let mut ok = 0usize;
    let mut failed = 0usize;
    for f in files {
        match resize_one_image(f, width, scale, quality, format, output) {
            Ok(()) => ok += 1,
            Err(e) => {
                failed += 1;
                ui::error(&format!("{f}: {e}"));
            }
        }
    }
    if files.len() > 1 {
        use owo_colors::OwoColorize;
        println!();
        println!(
            "  ✅ {}장 완료{}",
            ok.to_string().bright_white(),
            if failed > 0 {
                format!(", {failed}장 실패").red().to_string()
            } else {
                String::new()
            }
        );
        println!();
    }
    Ok(())
}

/// 이미지 한 장을 줄이거나/형식 변환해 _작게·_변환(또는 --출력)으로 저장한다.
fn resize_one_image(
    file: &str,
    width: Option<u32>,
    scale: Option<f64>,
    quality: u8,
    format: Option<&str>,
    output: Option<&str>,
) -> Result<()> {
    use owo_colors::OwoColorize;
    use std::path::{Path, PathBuf};
    let path = Path::new(file);
    if !path.exists() {
        anyhow::bail!("파일을 찾을 수 없어요: {file}");
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let src_jpeg = matches!(ext.as_str(), "jpg" | "jpeg");
    let src_png = ext == "png";
    if !src_jpeg && !src_png {
        anyhow::bail!("JPEG·PNG만 지원해요(받은 형식: .{ext}). HEIC·WebP는 아직 미지원이에요.");
    }
    // 목표 형식(--형식 우선, 없으면 원본 형식).
    let target_ext = match format {
        Some(f) => match f.to_lowercase().as_str() {
            "jpg" | "jpeg" => "jpg".to_string(),
            "png" => "png".to_string(),
            other => anyhow::bail!("형식은 jpg 또는 png만 돼요(받은 값: {other})."),
        },
        None => {
            if src_jpeg {
                "jpg".to_string()
            } else {
                "png".to_string()
            }
        }
    };
    let target_jpeg = target_ext == "jpg";
    let converting = (target_jpeg && !src_jpeg) || (!target_jpeg && !src_png);

    let orig_bytes = std::fs::metadata(path)?.len();
    let img = open_image_oriented(path)?;
    let (w, h) = (img.width(), img.height());
    let (nw, nh) = plan_resize(w, h, width, scale);

    let out_img = if (nw, nh) != (w, h) {
        img.resize(nw, nh, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // 출력 경로: 기본은 원본 옆에 접미사(형식 변환은 _변환, 아니면 _작게). 원본 절대 덮어쓰지 않음.
    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            let suffix = if converting { "_변환" } else { "_작게" };
            parent.join(format!("{stem}{suffix}.{target_ext}"))
        }
    };
    if out_path == path {
        anyhow::bail!("출력 경로가 원본과 같아요. 원본 보존을 위해 다른 경로(--출력)를 쓰세요.");
    }

    // 인코딩: JPEG는 품질 적용, PNG는 무손실 재인코딩.
    if target_jpeg {
        // 투명 부분은 흰 배경에 합성(검게 나오지 않도록).
        let rgb = flatten_on_white(&out_img);
        let f = std::fs::File::create(&out_path)?;
        let mut w = std::io::BufWriter::new(f);
        let q = quality.clamp(1, 100);
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, q);
        enc.encode_image(&rgb)?;
    } else {
        out_img.save(&out_path)?;
    }

    let new_bytes = std::fs::metadata(&out_path)?.len();
    let pct = if orig_bytes > 0 {
        100.0 - (new_bytes as f64 / orig_bytes as f64 * 100.0)
    } else {
        0.0
    };
    println!();
    println!("  🖼️  {}", file.bright_cyan());
    if (nw, nh) != (w, h) {
        println!(
            "     크기  {w}×{h}  →  {}",
            format!("{nw}×{nh}").bright_white()
        );
    } else {
        println!("     크기  {w}×{h} (유지)");
    }
    if converting {
        println!(
            "     형식  .{ext}  →  {}",
            format!(".{target_ext}").bright_white()
        );
    }
    println!(
        "     용량  {}  →  {}  ({})",
        human_bytes(orig_bytes),
        human_bytes(new_bytes).bright_white(),
        if pct >= 0.5 {
            format!("-{pct:.0}%").bright_green().to_string()
        } else {
            "변화 적음".dimmed().to_string()
        }
    );
    println!(
        "     저장  {}",
        out_path.display().to_string().bright_yellow()
    );
    println!();
    Ok(())
}

/// 현황 상단 "원장 한마디" — 로컬 데이터에서 지금 가장 신경 쓸 하나를 골라 먼저 말해준다.
/// (GPT가 못 하는 일: 사용자 로컬의 디데이·습관·약속·할일을 읽어 우선순위로 종합)
/// 우선순위: 임박 디데이(≤3일) > 끊길 위기 습관(streak≥2·오늘 미완) > 임박 약속(≤180분) > 남은 할 일.
fn status_highlight(
    nearest_dday: Option<(i64, String)>,
    at_risk_habit: Option<(i64, String)>,
    soon_reminder: Option<(i64, String)>,
    pending_todos: usize,
) -> Option<String> {
    if let Some((days, label)) = nearest_dday {
        if (0..=3).contains(&days) {
            return Some(match days {
                0 => format!("오늘은 '{label}' 당일이에요! 잊지 않으셨죠? 🎯"),
                1 => format!("내일이 '{label}'! 하루 남았어요 — 미리 챙겨둬요."),
                d => format!("'{label}'까지 D-{d}. 슬슬 준비하면 딱 좋아요."),
            });
        }
    }
    if let Some((streak, name)) = at_risk_habit {
        return Some(format!(
            "'{name}' {streak}일 연속 중인데 오늘 아직이에요 — 여기서 끊기면 아깝잖아요 🔥"
        ));
    }
    if let Some((mins, title)) = soon_reminder {
        let when = if mins <= 60 {
            format!("{mins}분 뒤")
        } else {
            format!("{}시간 뒤", mins / 60)
        };
        return Some(format!("{when} '{title}' 약속이 있어요. 준비됐어요?"));
    }
    if pending_todos > 0 {
        return Some(format!(
            "오늘 할 일 {pending_todos}개가 기다려요. 하나씩 같이 해봐요 💪"
        ));
    }
    None
}

/// 내 기록을 '자랑 카드' 한 장으로(카톡에 안 깨지는 ANSI 박스). 데이터는 읽기 전용.
fn cmd_brag(
    month: Option<&str>,
    week: bool,
    width: usize,
    no_color: bool,
    copy: bool,
) -> Result<()> {
    use owo_colors::OwoColorize;

    if week {
        return cmd_brag_weekly(width, no_color, copy);
    }

    // 대상 월(YYYY-MM)과 표시 제목. 한 자리 월("2026-6")도 0 채워 정규화(저장 형식과 일치).
    let ym = match month {
        Some(m) => match m.trim().split_once('-') {
            Some((y, mo)) => match mo.trim().parse::<u32>() {
                Ok(n) if (1..=12).contains(&n) => format!("{}-{n:02}", y.trim()),
                _ => m.trim().to_string(),
            },
            None => m.trim().to_string(),
        },
        None => expenses::this_month(),
    };
    let title = match ym.split_once('-') {
        Some((y, mo)) => format!("{y}년 {}월", mo.trim_start_matches('0')),
        None => ym.clone(),
    };

    // 습관: 가장 긴 streak + 그 습관의 최근 28일 잔디.
    let habit_store = habits::HabitStore::load().unwrap_or_default();
    let today = habits::today();
    let mut best: Option<(String, i64, std::collections::HashSet<String>)> = None;
    for h in &habit_store.items {
        let s = h.streak(today);
        if best.as_ref().map(|(_, bs, _)| s > *bs).unwrap_or(true) {
            best = Some((h.name.clone(), s, h.date_set()));
        }
    }
    let streak = best.as_ref().map(|(n, s, _)| (n.clone(), *s));
    let jandi: Vec<bool> = match &best {
        Some((_, _, set)) => (0..28)
            .rev()
            .map(|i| {
                let d = today - chrono::Duration::days(i);
                set.contains(&d.format("%Y-%m-%d").to_string())
            })
            .collect(),
        None => Vec::new(),
    };

    // 집중·지출(월 합계), 디데이(가까운 미래), 일기(이번 달 기록 수).
    let foc = focus::FocusStore::load().unwrap_or_default();
    let focus_min: i64 = (1..=31)
        .map(|day| foc.today_total(&format!("{ym}-{day:02}")))
        .sum();
    let exp = expenses::ExpenseStore::load().unwrap_or_default();
    let expense_won = exp.total_in_month(&ym);
    let dd = ddays::DdayStore::load().unwrap_or_default();
    let td = ddays::today();
    let dday = dd
        .all()
        .iter()
        .filter_map(|d| {
            ddays::parse_date(&d.date)
                .ok()
                .map(|dt| (ddays::days_until(dt, td), d.label.clone()))
        })
        .filter(|(days, _)| *days >= 0)
        .min_by_key(|(days, _)| *days)
        .map(|(days, label)| format!("{label} {}", ddays::dday_label(days)));
    let journal_count = journal::this_month().map(|v| v.len()).unwrap_or(0);

    // 데이터가 없으면 카드 대신 '시작하기' 한 줄(수집→자랑 루프의 출발점).
    let has_data = !habit_store.items.is_empty()
        || focus_min > 0
        || expense_won > 0
        || dday.is_some()
        || journal_count > 0;
    if !has_data {
        // 호기심에 처음 '자랑'을 친 사람에게 텍스트만 주면 '갖고 싶다'가 안 생긴다.
        // 예시 카드로 결과물을 먼저 보여줘 습관 등록 → 전염으로 잇는다(첫인상 전환).
        println!();
        ui::info("아직 쌓인 기록이 없어요 — 한 달만 채우면 이런 카드가 생겨요:");
        let sample = card::CardData {
            title: "예시".into(),
            streak: Some(("운동".into(), 15)),
            jandi: (0..28).map(|i| i % 4 != 0).collect(),
            focus_label: Some("12시간".into()),
            expense_label: Some("320,000원".into()),
            dday: Some("수능 D-150".into()),
            journal_count: 8,
            comment: "꾸준함이 멋져요 🙌".into(),
            footer: card::SHARE_FOOTER.into(),
        };
        print_card(
            &card::render_card(&sample, width),
            soul::active_preset_key(),
            no_color,
            false,
        );
        println!("  {}", "wonjang 습관 add 운동".bright_cyan());
        ui::info("  매일 'wonjang 습관 done 운동' → 한 달 뒤 진짜 내 카드가 생겨요 🌱");
        println!();
        return Ok(());
    }

    let persona = soul::active_preset_key();
    let comment = card::card_comment(
        persona,
        streak.as_ref().map(|(_, s)| *s).unwrap_or(0),
        streak.as_ref().map(|(n, _)| n.as_str()).unwrap_or("습관"),
    );
    let data = card::CardData {
        title,
        streak,
        jandi,
        focus_label: (focus_min > 0).then(|| focus::fmt_minutes(focus_min)),
        expense_label: (expense_won > 0).then(|| expenses::won(expense_won)),
        dday,
        journal_count,
        comment,
        footer: card::SHARE_FOOTER.to_string(),
    };

    print_card(&card::render_card(&data, width), persona, no_color, copy);
    Ok(())
}

/// 카드 줄들을 출력한다(테두리만 페르소나 테마색). `copy`면 플레인 텍스트를 클립보드에도.
fn print_card(lines: &[String], persona: &str, no_color: bool, copy: bool) {
    use owo_colors::{AnsiColors, OwoColorize};
    use std::io::IsTerminal;
    let color = !no_color && std::io::stdout().is_terminal();
    let theme = match persona {
        "친구" => AnsiColors::Green,
        "집사" => AnsiColors::Yellow,
        "선배" => AnsiColors::White,
        "발랄" => AnsiColors::Magenta,
        _ => AnsiColors::Cyan,
    };
    println!();
    for line in lines {
        let is_border = line.starts_with('╭') || line.starts_with('├') || line.starts_with('╰');
        if color && is_border {
            println!("{}", line.color(theme));
        } else {
            println!("{line}");
        }
    }
    if copy {
        // lines는 색 없는 플레인 문자열 → 그대로 클립보드에(카톡·메모 붙여넣기용).
        match clipboard::write(&lines.join("\n")) {
            Ok(_) => ui::note("  📋 클립보드에 복사했어요 — 카톡·메모에 붙여넣으세요!"),
            Err(e) => ui::error(&format!("클립보드 복사 실패: {e}")),
        }
    } else if color {
        ui::info("  카톡엔 --폭 34, 복사는 --복사 가 편해요");
    }
    println!();
}

/// 주간 자랑 카드(이번 주 + 지난주 대비 ▲▼). 데이터 읽기 전용.
fn cmd_brag_weekly(width: usize, no_color: bool, copy: bool) -> Result<()> {
    use chrono::Datelike;
    use owo_colors::OwoColorize;
    let today = ddays::today();
    let day = |d: chrono::NaiveDate| d.format("%Y-%m-%d").to_string();

    // 습관: 가장 긴 streak + 이번 주 7일 잔디.
    let habit_store = habits::HabitStore::load().unwrap_or_default();
    let mut best: Option<(String, i64, std::collections::HashSet<String>)> = None;
    for h in &habit_store.items {
        let s = h.streak(today);
        if best.as_ref().map(|(_, bs, _)| s > *bs).unwrap_or(true) {
            best = Some((h.name.clone(), s, h.date_set()));
        }
    }
    let streak = best.as_ref().map(|(n, s, _)| (n.clone(), *s));
    let jandi7: Vec<bool> = match &best {
        Some((_, _, set)) => (0..7)
            .rev()
            .map(|i| set.contains(&day(today - chrono::Duration::days(i))))
            .collect(),
        None => Vec::new(),
    };

    // 집중·지출: 이번 주(오늘-6..오늘) vs 지난주(오늘-13..오늘-7).
    let foc = focus::FocusStore::load().unwrap_or_default();
    let exp = expenses::ExpenseStore::load().unwrap_or_default();
    let (mut tw_f, mut lw_f, mut tw_e, mut lw_e) = (0i64, 0i64, 0i64, 0i64);
    for i in 0..7 {
        let d = today - chrono::Duration::days(i);
        let d2 = today - chrono::Duration::days(i + 7);
        tw_f += foc.today_total(&day(d));
        lw_f += foc.today_total(&day(d2));
        tw_e += exp.total_on(&day(d));
        lw_e += exp.total_on(&day(d2));
    }

    let has_data = !habit_store.items.is_empty() || tw_f > 0 || lw_f > 0 || tw_e > 0 || lw_e > 0;
    if !has_data {
        println!();
        ui::note("이번 주 자랑할 게 아직 없어요. 습관 하나만 시작해봐요:");
        println!("  {}", "wonjang 습관 add 운동".bright_cyan());
        println!();
        return Ok(());
    }

    let persona = soul::active_preset_key();
    let f_delta = tw_f - lw_f;
    let e_delta = tw_e - lw_e;
    let focus_value = if f_delta == 0 {
        focus::fmt_minutes(tw_f)
    } else {
        format!(
            "{}  {}{}",
            focus::fmt_minutes(tw_f),
            card::delta_arrow(f_delta),
            focus::fmt_minutes(f_delta.abs())
        )
    };
    let expense_value = if e_delta == 0 {
        expenses::won(tw_e)
    } else {
        format!(
            "{}  {}{}",
            expenses::won(tw_e),
            card::delta_arrow(e_delta),
            expenses::won(e_delta.abs())
        )
    };

    let start = today - chrono::Duration::days(6);
    let title = format!(
        "{}/{}~{}/{}",
        start.month(),
        start.day(),
        today.month(),
        today.day()
    );
    let data = card::WeeklyCardData {
        title,
        streak,
        jandi7,
        focus_value,
        expense_value,
        comment: card::weekly_comment(persona, f_delta),
        footer: card::SHARE_FOOTER.to_string(),
    };
    print_card(
        &card::render_weekly_card(&data, width),
        persona,
        no_color,
        copy,
    );
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
        soul::greeting().dimmed()
    );
    // 오늘 날짜·요일(매일 보는 대시보드의 기본).
    let now_local = chrono::Local::now().date_naive();
    println!(
        "  📅 {} ({})",
        now_local.format("%Y년 %-m월 %-d일"),
        datecalc::weekday_kr(now_local)
    );
    // 로컬 데이터를 먼저 읽어 "원장 한마디"로 종합(가장 신경 쓸 하나 먼저).
    let rem = reminders::ReminderStore::load()?;
    let upcoming = rem.upcoming(now_unix);
    let todo = todos::TodoStore::load()?;
    let dd = ddays::DdayStore::load()?;
    let habit = habits::HabitStore::load()?;

    let nearest_dday = dd
        .all()
        .iter()
        .filter_map(|d| {
            ddays::parse_date(&d.date)
                .ok()
                .map(|dt| (ddays::days_until(dt, today), d.label.clone()))
        })
        .filter(|(days, _)| *days >= 0)
        .min_by_key(|(days, _)| *days);
    let today_hs = habits::today_str();
    let at_risk_habit = habit
        .items
        .iter()
        .filter(|h| !h.done_today(&today_hs))
        .map(|h| (h.streak(today), h.name.clone()))
        .filter(|(s, _)| *s >= 2)
        .max_by_key(|(s, _)| *s);
    let soon_reminder = upcoming.first().and_then(|r| {
        let mins = (r.at_unix - now_unix) / 60;
        if (0..=180).contains(&mins) {
            Some((mins, r.title.clone()))
        } else {
            None
        }
    });
    if let Some(msg) = status_highlight(
        nearest_dday,
        at_risk_habit,
        soon_reminder,
        todo.pending().len(),
    ) {
        println!("  💬 {} {}", "원장".bright_cyan().bold(), msg);
    }
    println!();

    // 다가오는 약속(최대 3).
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
    let pending = todo.pending();
    println!("  ✅ 할 일 ({}개)", pending.len());
    for t in pending.iter().take(5) {
        println!("     ☐ {}", t.text);
    }
    if pending.len() > 5 {
        ui::info(&format!("     … 외 {}개", pending.len() - 5));
    }

    // 디데이(가까운 3).
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
    if birth > today {
        anyhow::bail!(
            "미래 날짜예요({}) — 생년월일을 다시 확인해 주세요.",
            birth.format("%Y-%m-%d")
        );
    }
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
    // 살아온 날수 + 다음 1000일 마디(한국에서 챙기는 '날수' 기념 — "나 1만일!" 자랑거리).
    let lived = age::days_lived(birth, today);
    println!("     🗓️ 살아온 날: {}일", exchange::comma(lived as f64, 0));
    if let Some((mark, date)) = age::next_day_milestone(birth, today) {
        let to = (date - today).num_days();
        println!(
            "     🎉 {}일까지 D-{to}  ({})",
            exchange::comma(mark as f64, 0),
            date.format("%Y-%m-%d")
        );
    }
    if dday == 0 {
        println!("     🎂 오늘이 생일이에요!");
    } else {
        println!("     다음 생일까지 {dday}일 남았어요");
    }
    println!();
    Ok(())
}

/// 단발 입력의 첫 단어가 프리셋 이름/별칭이면 그 프리셋으로 해석한다.
///
/// 반환: `Some((최종_프롬프트, 안내문구))` 또는 일치 없으면 `None`.
/// 클랩 서브커맨드는 이 경로에 닿기 전에 이미 처리되므로, 여기 첫 단어는 명령이 아니다
/// (압축·날씨 등 이름이 겹치는 프리셋은 클랩 명령이 가져가 충돌하지 않는다). 첫 단어가
/// 프리셋과 **정확히** 일치할 때만 발동해 자연어 오인을 막는다.
fn resolve_bare_preset(prompt: &[String]) -> Option<(String, String)> {
    let (first, rest) = prompt.split_first()?;
    let p = preset::find(first)?;
    let mut full = p.prompt;
    if !rest.is_empty() {
        full.push_str("\n\n추가 지시: ");
        full.push_str(&rest.join(" "));
    }
    let note = format!("프리셋 실행: {} — {}", p.name, p.description);
    Some((full, note))
}

/// 내보내기 파일의 기본 저장 위치를 정한다.
///
/// 사용자가 `--출력` 경로를 안 주면 **다운로드 폴더**에 저장한다 — `.ics`/`.csv`는
/// 받아서 더블클릭으로 캘린더·엑셀에 넣는 파일이라, 브라우저 내려받기처럼 다운로드
/// 폴더에서 찾는 게 자연스럽다. 예전엔 현재 폴더(cwd)에 상대 이름으로 떨궈, 어디
/// 저장됐는지 알기 어렵고 깃 저장소 같은 곳에서 실행하면 그 폴더를 어지럽혔다.
/// 다운로드 폴더가 없으면 종전대로 현재 폴더에 저장한다.
fn default_export_path(filename: &str) -> std::path::PathBuf {
    // 환경 의존부(실제 다운로드 폴더 존재 확인)는 여기서, 경로 결정 로직은
    // 순수 함수로 분리해 테스트한다.
    let dl = dirs::download_dir().filter(|d| d.is_dir());
    pick_export_path(dl, filename)
}

/// 다운로드 폴더(있으면)를 받아 내보내기 경로를 정한다. 폴더가 있으면 그 안에,
/// 없으면 현재 폴더에 파일 이름만으로.
fn pick_export_path(
    download_dir: Option<std::path::PathBuf>,
    filename: &str,
) -> std::path::PathBuf {
    match download_dir {
        Some(dl) => dl.join(filename),
        None => std::path::PathBuf::from(filename),
    }
}

fn cmd_salary(manwon: f64) -> Result<()> {
    if !manwon.is_finite() || manwon <= 0.0 {
        anyhow::bail!("연봉은 1만원 이상이어야 해요. 예: wonjang 실수령 3600");
    }
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
    if months == 0 {
        anyhow::bail!("개월 수는 1 이상이어야 해요. 예: wonjang 대출 30000 4.5 360");
    }
    // 음수 금리·원금(`-- -4.5` 등)은 음수 이자처럼 무의미 출력을 내므로 거부.
    if rate < 0.0 || manwon < 0.0 {
        anyhow::bail!("원금·연이율은 0 이상이어야 해요.");
    }
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
    if months == 0 {
        anyhow::bail!("개월 수는 1 이상이어야 해요. 예: wonjang 예금 1000 3.5 12");
    }
    if rate < 0.0 || manwon < 0.0 {
        anyhow::bail!("원금·연이율은 0 이상이어야 해요.");
    }
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

fn cmd_jeonse(jeonse_manwon: f64, rate_pct: f64, deposit_manwon: f64) -> Result<()> {
    if !jeonse_manwon.is_finite() || jeonse_manwon <= 0.0 {
        anyhow::bail!("전세보증금은 0보다 커야 해요. 예: wonjang 전월세 30000 5.5 (만원 단위)");
    }
    if !rate_pct.is_finite() || rate_pct < 0.0 {
        anyhow::bail!("전환율은 0 이상이어야 해요(법정 상한=기준금리+2%).");
    }
    if !deposit_manwon.is_finite() || deposit_manwon < 0.0 || deposit_manwon > jeonse_manwon {
        anyhow::bail!("월세 보증금은 0~전세금 사이여야 해요.");
    }
    let won = |man: f64| expenses::won((man * 10_000.0).round() as i64);
    let monthly = jeonse::monthly_rent(jeonse_manwon, deposit_manwon, rate_pct);
    let full = jeonse::monthly_rent(jeonse_manwon, 0.0, rate_pct);
    println!();
    println!(
        "  🏠 전월세 전환 (전세 {} · 전환율 {rate_pct}%)",
        jeonse::fmt_eok(jeonse_manwon)
    );
    if deposit_manwon > 0.0 {
        println!(
            "     보증금 {} → 월세 약 {:.1}만원 ({})",
            jeonse::fmt_eok(deposit_manwon),
            monthly,
            won(monthly)
        );
    }
    println!("     순수 월세(보증금 0) → {:.1}만원 ({})", full, won(full));
    ui::info("     ※ 법정 상한 = 한국은행 기준금리 + 2%");
    println!();
    Ok(())
}

fn cmd_severance(monthly_manwon: f64, years: u32, months: u32) -> Result<()> {
    if !monthly_manwon.is_finite() || monthly_manwon <= 0.0 {
        anyhow::bail!("월 평균임금은 0보다 커야 해요. 예: wonjang 퇴직금 300 3 (만원·근속년)");
    }
    if months >= 12 {
        anyhow::bail!("개월은 0~11로 넣어주세요(연수는 첫 숫자). 예: 3년 6개월 → 퇴직금 300 3 6");
    }
    let days = severance::service_days(years, months);
    println!();
    println!(
        "  💼 퇴직금 추정 (월 평균임금 {}만원 · 근속 {years}년 {months}개월)",
        monthly_manwon as i64
    );
    if days < 365 {
        ui::info("     근속 1년 미만은 법정 퇴직금 대상이 아니에요(1년 이상부터).");
        println!();
        return Ok(());
    }
    let sev = severance::severance_manwon(monthly_manwon, days);
    let won = expenses::won((sev * 10_000.0).round() as i64);
    println!("     예상 퇴직금   약 {:.0}만원 ({won})", sev);
    ui::info("     ※ 1일 평균임금 × 30일 × (재직일수/365). 평균임금엔 상여·연차수당이");
    ui::info("       더해지므로 실제 퇴직금은 이보다 많을 수 있어요(추정치).");
    println!();
    Ok(())
}

fn cmd_annual_leave(years: u32, months: u32) -> Result<()> {
    if months >= 12 {
        anyhow::bail!("개월은 0~11로 넣어주세요(연수는 첫 숫자). 예: 1년 6개월 → 연차 1 6");
    }
    let days = annual_leave::annual_leave_days(years, months);
    println!();
    println!("  🌴 연차 휴가 (근속 {years}년 {months}개월)");
    println!("     발생 연차   {days}일");
    if years == 0 {
        ui::info("     1개월 개근당 1일(최대 11일). 만 1년이 되면 15일로 새로 발생해요.");
    } else {
        match annual_leave::next_increase(years) {
            Some((yr, d)) => ui::info(&format!(
                "     {yr}년차에 {d}일로 늘어요(3년부터 매 2년 +1일, 최대 25일)."
            )),
            None => ui::info("     법정 상한(25일)에 도달했어요."),
        }
    }
    println!();
    Ok(())
}

fn cmd_car_tax(cc: u32, age: u32) -> Result<()> {
    if cc == 0 {
        anyhow::bail!("배기량(cc)을 입력하세요. 예: wonjang 자동차세 1998 [차령]");
    }
    let (tax, edu) = car_tax::annual_tax(cc, age);
    let total = tax + edu;
    let won = |v: i64| expenses::won(v);
    println!();
    println!("  🚗 자동차세 (비영업 승용 {cc}cc · 차령 {age}년)");
    println!("     자동차세      {}", won(tax));
    println!("     지방교육세    {}  (자동차세의 30%)", won(edu));
    println!("     ───────────────");
    println!("     연 세액       {}", won(total));
    ui::info(&format!(
        "     6월·12월 각 {} 납부(연납 신청 시 일부 공제).",
        won(total / 2)
    ));
    if age < 3 {
        ui::info("     ※ 차령 3년부터 매년 5%씩 경감(최대 50%).");
    }
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

fn cmd_uptime(url: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let u = url.to_string();
    let status = util::run_async(async move { uptime::check(&u).await })?;
    let code_str = match status.code {
        200..=299 => format!("{}", status.code).green().to_string(),
        300..=399 => format!("{}", status.code).yellow().to_string(),
        _ => format!("{}", status.code).red().to_string(),
    };
    let speed = match status.elapsed_ms {
        0..=300 => "빠름".green().to_string(),
        301..=1000 => "보통".yellow().to_string(),
        _ => "느림".red().to_string(),
    };
    println!();
    if status.ok {
        println!("  ✅ 정상 — {}", status.url.bright_cyan());
    } else {
        println!("  ⚠️ 응답함(비정상 코드) — {}", status.url.bright_cyan());
    }
    println!("     상태 코드: {code_str}");
    println!("     응답 시간: {}ms ({speed})", status.elapsed_ms);
    println!();
    Ok(())
}

fn cmd_myip() -> Result<()> {
    use owo_colors::OwoColorize;
    let info = util::run_async(async move { myip::fetch().await })?;
    println!();
    println!("  🌐 내 공인 IP");
    println!("     {}", info.ip.bright_cyan().bold());
    let loc: Vec<&str> = [
        info.country.as_str(),
        info.region.as_str(),
        info.city.as_str(),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect();
    if !loc.is_empty() {
        println!("     위치: {}", loc.join(" "));
    }
    if !info.isp.is_empty() {
        println!("     통신사: {}", info.isp);
    }
    if !info.org.is_empty() && info.org != info.isp {
        println!("     조직: {}", info.org.dimmed());
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

fn cmd_journal(text: &[String]) -> Result<()> {
    use owo_colors::OwoColorize;
    // 내용이 있으면 기록.
    if !text.is_empty() {
        journal::add(&text.join(" "))?;
        println!();
        println!("  📔 일기에 기록했어요.");
        println!();
        return Ok(());
    }
    // 없으면 이번 달 보기.
    let entries = journal::this_month()?;
    println!();
    println!(
        "  📔 {} 일기 ({}건)",
        chrono::Local::now().format("%Y년 %-m월"),
        entries.len()
    );
    if entries.is_empty() {
        println!(
            "     아직 기록이 없어요. {}",
            "wonjang 일기 \"오늘 있었던 일\"".dimmed()
        );
        println!();
        return Ok(());
    }
    for e in entries.iter().take(20) {
        println!();
        println!("  {}", e.stamp.bright_cyan());
        for line in e.text.lines() {
            println!("     {line}");
        }
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

    // 다음 연휴(주말+공휴일이 3일 이상 연속) — "추석 며칠 쉬어?"를 한 줄로(올해 한정).
    if year == chrono::Datelike::year(&today) {
        if let Some((start, end, len)) = holidays::next_long_break(&holidays, today, 3) {
            let wd = |d: chrono::NaiveDate| {
                format!("{}({})", d.format("%m-%d"), datecalc::weekday_kr(d))
            };
            let dday = datecalc::days_between(today, start);
            let when = if dday <= 0 {
                "지금".to_string()
            } else {
                format!("D-{dday}")
            };
            println!(
                "  🏖️ 다음 연휴: {}~{} {len}일 연속 ({when})",
                wd(start),
                wd(end)
            );
        }
        // 연차 하나로 만드는 황금연휴(징검다리) — 한국에서 연말마다 가장 많이 공유되는 정보.
        let golden = holidays::golden_leaves(&holidays, today, 4, 3);
        if !golden.is_empty() {
            let wd = |d: chrono::NaiveDate| {
                format!("{}({})", d.format("%m-%d"), datecalc::weekday_kr(d))
            };
            println!("  💡 연차 하나로 황금연휴:");
            for g in &golden {
                println!(
                    "     {} 연차 → {}일 연휴 ({}~{})",
                    wd(g.leave),
                    g.len,
                    wd(g.start),
                    wd(g.end)
                );
            }
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
        let result = datecalc::add_days(base, n).ok_or_else(|| {
            anyhow::anyhow!("{n}일은 너무 커서 날짜 범위를 벗어나요. 더 작은 값을 쓰세요.")
        })?;
        let word = if n >= 0 { "후" } else { "전" };
        println!("  📅 {} 기준 {}일 {word}", fmt(base), n.abs());
        println!("     👉 {}", fmt(result));
    } else {
        // 날짜 하나만: 오늘 기준 관계를 알려준다.
        // 과거면 '며칠째 + 다가오는 기념일'(사귄 날·기념일 계산), 미래면 D-day, 오늘이면 오늘.
        println!("  📅 {}", fmt(base));
        let diff = datecalc::days_between(today, base); // base가 미래면 +, 과거면 -
        if diff > 0 {
            println!("     D-{diff}  ({diff}일 남음)");
        } else if diff < 0 {
            let s = datecalc::days_since(base, today);
            println!(
                "     오늘로 {}일째  ({}일 전 · 사귄 날·기념일 계산)",
                s.nth_day, s.days_ago
            );
            for (mark, date, dday) in &s.milestones {
                println!("     🎉 {mark}일 → {} (D-{dday})", fmt(*date));
            }
        } else {
            let day_of_year = chrono::Datelike::ordinal(&base);
            println!("     올해 {day_of_year}번째 날 (오늘)");
        }
    }
    println!();
    Ok(())
}

fn cmd_encode(method: &str, text: &[String], decode: bool) -> Result<()> {
    use owo_colors::OwoColorize;
    if text.is_empty() {
        println!();
        println!("  대상 텍스트를 입력하세요. 예: wonjang 인코딩 base64 \"hello\"");
        println!();
        return Ok(());
    }
    let input = text.join(" ");
    let result = match (method.trim().to_lowercase().as_str(), decode) {
        ("base64" | "b64", false) => encode::base64_encode(&input),
        ("base64" | "b64", true) => encode::base64_decode(&input)?,
        ("url", false) => encode::url_encode(&input),
        ("url", true) => encode::url_decode(&input)?,
        (other, _) => {
            return Err(anyhow::anyhow!(
                "방식은 base64 또는 url 이어야 해요 (입력: {other})"
            ))
        }
    };
    let action = if decode { "디코딩" } else { "인코딩" };
    println!();
    println!("  🔠 {} {action}", method.to_lowercase().bright_cyan());
    println!("     {result}");
    println!();
    Ok(())
}

fn cmd_tzconv(time: &str, from: &str, to: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let c = worldtime::convert(time, from, to)?;
    println!();
    println!("  🕑 시간대 변환");
    println!(
        "     {}  →  {} {}",
        c.from_label.bold(),
        c.to_label.bright_cyan().bold(),
        c.day_note.dimmed()
    );
    println!();
    Ok(())
}

fn cmd_worldtime(city: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    let cities = worldtime::lookup(city);
    println!();
    if cities.is_empty() {
        println!(
            "  '{}' 도시를 못 찾았어요. 서울/뉴욕/런던/도쿄 등으로 검색하세요.",
            city.unwrap_or("")
        );
        println!();
        return Ok(());
    }
    println!("  🌏 세계 시간");
    for c in &cities {
        println!(
            "     {:<8} {}   {}",
            c.name.bold(),
            c.time,
            c.offset.dimmed()
        );
    }
    println!();
    Ok(())
}

fn cmd_timestamp(value: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    println!();
    match value {
        None => {
            let c = timestamp::now();
            println!("  ⏰ 현재 시각");
            println!(
                "     유닉스(초):   {}",
                c.unix_sec.to_string().bright_cyan()
            );
            println!("     유닉스(밀리): {}", (c.unix_sec * 1000));
            println!("     로컬:         {}", c.local);
            println!("     UTC:          {}", c.utc.dimmed());
        }
        Some(v) => {
            let c = timestamp::convert(v)?;
            println!("  ⏰ 타임스탬프 변환");
            println!(
                "     유닉스(초):   {}",
                c.unix_sec.to_string().bright_cyan()
            );
            println!("     로컬:         {}", c.local);
            println!("     UTC:          {}", c.utc.dimmed());
        }
    }
    println!();
    Ok(())
}

fn cmd_roman(value: &str) -> Result<()> {
    use owo_colors::OwoColorize;
    let v = value.trim();
    println!();
    if v.chars().all(|c| c.is_ascii_digit()) {
        let n: u32 = v.parse().map_err(|_| anyhow::anyhow!("숫자가 너무 커요"))?;
        println!("  🏛️  {n} = {}", roman::to_roman(n)?.bright_cyan().bold());
    } else {
        let n = roman::from_roman(v)?;
        println!(
            "  🏛️  {} = {}",
            v.to_uppercase(),
            n.to_string().bright_cyan().bold()
        );
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
    // u64를 i64로 캐스트하면 2^63 이상에서 음수로 깨지므로(한글 줄과 모순·디버그 패닉) 직접 콤마 포맷.
    let s = value.to_string();
    let n = s.len();
    let mut commad = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (n - i).is_multiple_of(3) {
            commad.push(',');
        }
        commad.push(ch);
    }
    println!();
    println!("  💴 한글 금액");
    println!("     {commad}원");
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

/// 글자수 제한 대비 안내 문구. 딱 맞으면(=제한) "0자 남음". 순수.
fn char_limit_note(count: usize, limit: Option<usize>) -> String {
    match limit {
        Some(lim) if count <= lim => format!("  (제한 {lim} → {}자 남음)", lim - count),
        Some(lim) => format!("  (제한 {lim} → {}자 초과 ⚠️)", count - lim),
        None => String::new(),
    }
}

fn cmd_chars(text: &[String], limit: Option<usize>) -> Result<()> {
    println!();
    if text.is_empty() {
        println!("  셀 텍스트를 입력하세요. 예: wonjang 글자수 \"자기소개서 내용\"");
        println!();
        return Ok(());
    }
    let joined = text.join(" ");
    let c = charcount::count(&joined);
    // 자소서 제한이 주어지면 남은/초과 글자를 함께(포털마다 공백 포함/제외 기준이 달라 둘 다).
    let note = |count: usize| char_limit_note(count, limit);
    println!("  ✍️  글자수 세기");
    println!(
        "     공백 포함      {}자{}",
        c.chars_with_space,
        note(c.chars_with_space)
    );
    println!(
        "     공백 제외      {}자{}",
        c.chars_without_space,
        note(c.chars_without_space)
    );
    println!("     단어 수        {}개", c.words);
    println!("     줄 수          {}줄", c.lines);
    println!("     바이트         {}B", c.bytes);
    println!();
    Ok(())
}

fn cmd_wage(hourly: f64, weekly_hours: f64) -> Result<()> {
    if !hourly.is_finite() || !weekly_hours.is_finite() || hourly <= 0.0 || weekly_hours <= 0.0 {
        anyhow::bail!("시급과 주당 근무시간은 0보다 커야 해요. 예: wonjang 시급 10030 40");
    }
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
    if !amount.is_finite() || amount <= 0.0 {
        anyhow::bail!("금액은 0보다 커야 해요. 예: wonjang 부가세 11000");
    }
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

fn cmd_sleep(wake: Option<&str>) -> Result<()> {
    use owo_colors::OwoColorize;
    println!();
    match wake {
        Some(w) => {
            let wt = sleepcalc::parse_time(w)?;
            println!("  😴 {} 기상 기준 — 이 시각에 주무세요", w.bright_cyan());
            for (cycles, bed) in sleepcalc::bedtimes_for_wake(wt) {
                println!(
                    "     {}  ({}주기 · 약 {} 수면)",
                    bed.bold(),
                    cycles,
                    sleep_dur(cycles)
                );
            }
        }
        None => {
            use chrono::Timelike;
            let now = chrono::Local::now().time();
            let nt = chrono::NaiveTime::from_hms_opt(now.hour(), now.minute(), 0).unwrap();
            println!(
                "  😴 지금({}) 자면 — 이 시각에 일어나세요",
                sleepcalc_fmt(nt).dimmed()
            );
            for (cycles, wakeup) in sleepcalc::waketimes_for_bed(nt) {
                println!(
                    "     {}  ({}주기 · 약 {} 수면)",
                    wakeup.bold(),
                    cycles,
                    sleep_dur(cycles)
                );
            }
        }
    }
    println!();
    println!(
        "  {} 90분 수면 주기 끝에 깨면 개운해요(참고용).",
        "ⓘ".dimmed()
    );
    println!();
    Ok(())
}

/// 주기 수 → "N시간"/"N시간 30분" (90분 주기라 홀수 주기는 30분이 붙는다).
fn sleep_dur(cycles: u32) -> String {
    let total = cycles * 90;
    let (h, m) = (total / 60, total % 60);
    if m == 0 {
        format!("{h}시간")
    } else {
        format!("{h}시간 {m}분")
    }
}

fn sleepcalc_fmt(t: chrono::NaiveTime) -> String {
    use chrono::Timelike;
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn cmd_calorie(sex: &str, age: u32, height: f64, weight: f64) -> Result<()> {
    use owo_colors::OwoColorize;
    let sex = bmr::Sex::parse(sex)?;
    if !(1..=120).contains(&age) || height <= 0.0 || weight <= 0.0 {
        return Err(anyhow::anyhow!("나이·키·몸무게를 올바르게 입력하세요"));
    }
    let b = bmr::bmr(sex, age, height, weight);
    println!();
    println!("  🍚 칼로리 계산 ({age}세 · {height:.0}cm · {weight:.0}kg)");
    println!(
        "     기초대사량(BMR)  {} kcal/일",
        format!("{b:.0}").bright_cyan().bold()
    );
    println!();
    println!("  활동 수준별 하루 권장 칼로리(TDEE):");
    for (name, mult, desc) in bmr::ACTIVITY {
        println!(
            "     {:<8} {} kcal  ({})",
            name,
            format!("{:.0}", b * mult).bold(),
            desc.dimmed()
        );
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
        None => {
            // 평·면적은 한국에서 제일 흔한 변환이라 전용 명령으로 안내(convert엔 평이 없음).
            let u = unit.trim();
            if matches!(u, "평" | "평수" | "pyeong" | "py") {
                let v = if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    format!("{value}")
                };
                println!("  📐 평↔㎡ 변환은 이걸 쓰세요:  wonjang 평 {v}");
            } else {
                println!("  '{unit}' 단위는 몰라요. 가능: {}", convert::supported());
            }
        }
    }
    println!();
    Ok(())
}

fn cmd_color(input: &[String]) -> Result<()> {
    use owo_colors::OwoColorize;
    if input.is_empty() {
        println!();
        println!("  색을 입력하세요. 예: wonjang 색 #ff5733  또는  wonjang 색 255 87 51");
        println!();
        return Ok(());
    }
    // RGB 3개 숫자인지, 헥스인지 판별.
    let rgb = if input.len() == 3 && input.iter().all(|s| s.parse::<u16>().is_ok()) {
        let n: Vec<u16> = input.iter().map(|s| s.parse().unwrap()).collect();
        if n.iter().any(|&v| v > 255) {
            return Err(anyhow::anyhow!("RGB 값은 0~255 사이여야 해요"));
        }
        color::Rgb {
            r: n[0] as u8,
            g: n[1] as u8,
            b: n[2] as u8,
        }
    } else {
        color::parse_hex(&input.join(""))?
    };
    let (h, s, l) = color::to_hsl(rgb);
    println!();
    println!("  🎨 색상 변환");
    println!("     HEX  {}", color::to_hex(rgb).bright_cyan());
    println!("     RGB  rgb({}, {}, {})", rgb.r, rgb.g, rgb.b);
    println!("     HSL  hsl({:.0}, {:.0}%, {:.0}%)", h, s, l);
    println!();
    Ok(())
}

fn cmd_uuid(count: usize) -> Result<()> {
    use owo_colors::OwoColorize;
    let n = count.clamp(1, 50);
    println!();
    for _ in 0..n {
        println!("  {}", uuidgen::v4()?.bright_cyan());
    }
    println!();
    Ok(())
}

fn cmd_password(length: Option<usize>, symbols: bool, count: usize) -> Result<()> {
    use owo_colors::OwoColorize;
    let len = length.unwrap_or(16);
    let n = count.clamp(1, 20);
    println!();
    println!(
        "  🔑 비밀번호 ({}자{})",
        len.clamp(4, 128),
        if symbols { ", 기호 포함" } else { "" }
    );
    for _ in 0..n {
        let pw = password::generate(len, symbols)?;
        println!("     {}", pw.bright_cyan().bold());
    }
    println!();
    println!("  {} OS 난수로 생성(암호학적으로 안전).", "ⓘ".dimmed());
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
    ui::info(&format!("     {}", exchange::format_update_time(&date)));
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
            rem.add(
                reminders::now_unix().saturating_add(m.saturating_mul(60)),
                &title,
                None,
            )?;

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
            Some((name, streak)) => {
                ui::note(&format!("'{name}' 완료! 🔥 {streak}일 연속"));
                // 마일스톤이면 축하 + 자랑 카드 공유 유도(감정의 정점에서 전염 트리거).
                if let Some(label) = habits::milestone(streak) {
                    use owo_colors::OwoColorize;
                    println!(
                        "  🎉 {} {} 이 기록, 카드로 남겨 자랑해요 → {}",
                        format!("{name} {label}").bright_yellow().bold(),
                        "달성!".bright_yellow(),
                        "wonjang 자랑 --복사".bright_cyan()
                    );
                }
            }
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
        Some(ExpenseAction::Export { output }) => {
            use owo_colors::OwoColorize;
            use std::path::{Path, PathBuf};
            if store.items.is_empty() {
                ui::info("내보낼 지출 기록이 없어요.");
                return Ok(());
            }
            let headers = vec![
                "날짜".to_string(),
                "분류".into(),
                "금액".into(),
                "메모".into(),
            ];
            let rows: Vec<Vec<String>> = store
                .items
                .iter()
                .map(|e| {
                    vec![
                        e.date.clone(),
                        e.category.clone(),
                        e.amount.to_string(), // 콤마 없는 원시 숫자(엑셀에서 숫자로 인식)
                        e.note.clone(),
                    ]
                })
                .collect();
            let csv = sheet::to_csv(&headers, &rows);
            let out_path = match output {
                Some(o) => PathBuf::from(o),
                None => default_export_path("가계부.csv"),
            };
            util::atomic_write(Path::new(&out_path), csv.as_bytes())
                .map_err(|e| anyhow::anyhow!("저장 실패: {} ({e})", out_path.display()))?;
            println!();
            println!(
                "  💾 가계부 {}건을 CSV로 내보냈어요 → {}",
                store.items.len(),
                out_path.display().to_string().bright_yellow()
            );
            ui::note(&format!(
                "     분석: wonjang 엑셀 {} --그룹 분류 --열 금액",
                out_path.display()
            ));
            println!();
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
            // 이번 달 분류별 상위 항목(한눈에).
            let by = store.by_category_in_month(&ym);
            if !by.is_empty() {
                println!("\n  이번 달 분류별:");
                for (cat, amt) in by.iter().take(5) {
                    println!("     {cat:<8} {}", expenses::won(*amt));
                }
            }
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

/// D-day 카드 하단 한마디(남은 일수에 따라). 순수.
fn dday_card_comment(days: i64) -> String {
    match days {
        0 => "오늘이다! 끝까지 가보자 🎯".to_string(),
        d if (1..=7).contains(&d) => "막판 스퍼트! 💪".to_string(),
        d if (8..=30).contains(&d) => "한 발씩, 거의 왔어요 🔥".to_string(),
        d if d > 30 => "차근차근 준비해봐요 🌱".to_string(),
        d => format!("D+{}, 그동안 고생 많았어요 👏", -d),
    }
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
        Some(DdayAction::Export { output }) => {
            use owo_colors::OwoColorize;
            use std::path::{Path, PathBuf};
            if store.all().is_empty() {
                ui::info("내보낼 디데이가 없어요. 추가: wonjang 디데이 add \"수능\" 2026-11-19");
                return Ok(());
            }
            let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
            let ics = ddays::to_ics(store.all(), &stamp);
            let out_path = match output {
                Some(o) => PathBuf::from(o),
                None => default_export_path("디데이.ics"),
            };
            util::atomic_write(Path::new(&out_path), ics.as_bytes())
                .map_err(|e| anyhow::anyhow!("저장 실패: {} ({e})", out_path.display()))?;
            println!();
            println!(
                "  📅 디데이 {}개를 캘린더 파일로 내보냈어요 → {}",
                store.all().len(),
                out_path.display().to_string().bright_yellow()
            );
            ui::note("     이 파일을 열면(더블클릭) 구글·애플 캘린더에 종일 일정으로 들어가요.");
            println!();
        }
        Some(DdayAction::Card {
            name,
            width,
            no_color,
            copy,
        }) => {
            let today = ddays::today();
            let all = store.all();
            if all.is_empty() {
                ui::info("등록된 디데이가 없어요. 추가: wonjang dday add \"수능\" 2026-11-19");
                return Ok(());
            }
            // 이름이 주어지면 부분 일치, 아니면 가장 가까운 다가오는(없으면 가장 가까운) 디데이.
            let with_days: Vec<(&ddays::Dday, i64)> = all
                .iter()
                .filter_map(|d| {
                    ddays::parse_date(&d.date)
                        .ok()
                        .map(|dt| (d, ddays::days_until(dt, today)))
                })
                .collect();
            let chosen = match name {
                Some(n) => with_days.iter().find(|(d, _)| d.label.contains(n.as_str())),
                None => with_days
                    .iter()
                    .filter(|(_, days)| *days >= 0)
                    .min_by_key(|(_, days)| *days)
                    .or_else(|| with_days.iter().min_by_key(|(_, days)| days.abs())),
            };
            let Some((d, days)) = chosen.copied() else {
                ui::error(&format!(
                    "'{}' 디데이를 찾지 못했어요. 목록: wonjang 디데이",
                    name.as_deref().unwrap_or("")
                ));
                return Ok(());
            };
            let date = ddays::parse_date(&d.date)?;
            let date_line = format!("📅 {} ({})", d.date, datecalc::weekday_kr(date));
            let lines = card::render_dday_card(
                &d.label,
                &ddays::dday_label(days),
                &date_line,
                &dday_card_comment(days),
                *width,
            );
            print_card(&lines, soul::active_preset_key(), *no_color, *copy);
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
        let at = now.saturating_add(minutes.saturating_mul(60));
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

#[cfg(test)]
mod status_tests {
    use super::status_highlight;

    #[test]
    fn dday_takes_priority_and_only_within_3_days() {
        // D-2 디데이가 있으면 그것을 먼저.
        let msg = status_highlight(Some((2, "이사".into())), None, None, 5).unwrap();
        assert!(msg.contains("이사") && msg.contains("D-2"));
        // 당일.
        let msg = status_highlight(Some((0, "발표".into())), None, None, 0).unwrap();
        assert!(msg.contains("당일"));
        // 4일 뒤면 디데이는 건너뛰고 다음 우선순위(할 일)로.
        let msg = status_highlight(Some((4, "이사".into())), None, None, 3).unwrap();
        assert!(msg.contains("할 일") && !msg.contains("이사"));
    }

    #[test]
    fn at_risk_habit_beats_reminder_and_todo() {
        let msg =
            status_highlight(None, Some((5, "운동".into())), Some((30, "회의".into())), 9).unwrap();
        assert!(msg.contains("운동") && msg.contains('5'));
    }

    #[test]
    fn reminder_then_todo_fallback() {
        let msg = status_highlight(None, None, Some((30, "회의".into())), 4).unwrap();
        assert!(msg.contains("회의") && msg.contains("30분"));
        let msg = status_highlight(None, None, Some((120, "병원".into())), 4).unwrap();
        assert!(msg.contains("2시간"));
        let msg = status_highlight(None, None, None, 4).unwrap();
        assert!(msg.contains('4') && msg.contains("할 일"));
    }

    #[test]
    fn nothing_to_say_returns_none() {
        assert!(status_highlight(None, None, None, 0).is_none());
    }
}

#[cfg(test)]
mod habit_nudge_tests {
    use super::habit_evening_nudge;

    #[test]
    fn none_when_empty() {
        assert!(habit_evening_nudge(&[]).is_none());
    }

    #[test]
    fn single_habit_mentions_name_and_streak() {
        let msg = habit_evening_nudge(&[(5, "운동".into())]).unwrap();
        assert!(msg.contains("운동") && msg.contains('5') && !msg.contains("외 "));
    }

    #[test]
    fn multiple_habits_summarize_count() {
        let msg = habit_evening_nudge(&[(7, "독서".into()), (3, "운동".into()), (2, "물".into())])
            .unwrap();
        // 가장 센 streak를 대표로, 나머지는 개수로.
        assert!(msg.contains("독서") && msg.contains("외 2개"));
    }
}

#[cfg(test)]
mod pdfmerge_tests {
    use super::merge_documents;
    use lopdf::{dictionary, Document, Object, Stream};

    // n페이지짜리 최소 PDF Document를 메모리에 만든다.
    fn make_doc(pages: usize) -> Document {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut kids: Vec<Object> = Vec::new();
        for _ in 0..pages {
            let content_id = doc.add_object(Stream::new(dictionary! {}, b"".to_vec()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
                "Contents" => content_id,
            });
            kids.push(page_id.into());
        }
        let count = kids.len() as u32;
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => count,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc
    }

    #[test]
    fn merges_page_counts() {
        // 3페이지 + 2페이지 → 5페이지.
        let (merged, count) = merge_documents(vec![make_doc(3), make_doc(2)]).unwrap();
        assert_eq!(count, 5);
        assert_eq!(merged.get_pages().len(), 5);
    }

    #[test]
    fn single_document_passthrough() {
        let (merged, count) = merge_documents(vec![make_doc(4)]).unwrap();
        assert_eq!(count, 4);
        assert_eq!(merged.get_pages().len(), 4);
    }
}

#[cfg(test)]
mod pdfrotate_tests {
    use super::normalize_rotation;

    #[test]
    fn normalizes_multiples_of_90() {
        assert_eq!(normalize_rotation(90).unwrap(), 90);
        assert_eq!(normalize_rotation(270).unwrap(), 270);
        assert_eq!(normalize_rotation(360).unwrap(), 0);
        assert_eq!(normalize_rotation(450).unwrap(), 90);
        // 음수(반시계) → 양수로.
        assert_eq!(normalize_rotation(-90).unwrap(), 270);
    }

    #[test]
    fn rejects_non_multiples() {
        assert!(normalize_rotation(45).is_err());
        assert!(normalize_rotation(100).is_err());
    }
}

#[cfg(test)]
mod pdfpages_tests {
    use super::parse_page_range;

    #[test]
    fn parses_ranges_and_singles_sorted_unique() {
        assert_eq!(parse_page_range("1-3,5", 10).unwrap(), vec![1, 2, 3, 5]);
        // 중복·뒤섞임·공백 정리.
        assert_eq!(parse_page_range("5, 1-2, 2", 10).unwrap(), vec![1, 2, 5]);
        // 거꾸로 된 범위도 허용(스왑).
        assert_eq!(parse_page_range("3-1", 10).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn rejects_out_of_bounds_and_zero() {
        assert!(parse_page_range("1-3,8", 5).is_err()); // 8 > total 5
        assert!(parse_page_range("0", 5).is_err()); // 0 없음
        assert!(parse_page_range("", 5).is_err()); // 빈 지정
        assert!(parse_page_range("abc", 5).is_err()); // 숫자 아님
    }
}

#[cfg(test)]
mod photospdf_tests {
    use super::is_supported_image_ext;

    #[test]
    fn accepts_jpeg_png_case_insensitive() {
        assert!(is_supported_image_ext("a.jpg"));
        assert!(is_supported_image_ext("a.JPEG"));
        assert!(is_supported_image_ext("사진.PNG"));
        assert!(!is_supported_image_ext("a.heic"));
        assert!(!is_supported_image_ext("a.pdf"));
        assert!(!is_supported_image_ext("noext"));
    }
}

#[cfg(test)]
mod encfix_tests {
    use super::{cp949_to_utf8, looks_like_korean_text, utf8_to_cp949};

    #[test]
    fn cp949_roundtrip_recovers_korean() {
        let original = "안녕하세요 가계부 1월\n월세,500000";
        // UTF-8 → CP949 바이트 → 다시 UTF-8로 복구하면 동일해야 한다.
        let (cp949, lossy) = utf8_to_cp949(original);
        assert!(!lossy);
        assert_ne!(cp949, original.as_bytes()); // 바이트는 달라야(실제로 인코딩됨)
        let (recovered, had_errors) = cp949_to_utf8(&cp949);
        assert!(!had_errors);
        assert_eq!(recovered, original);
    }

    #[test]
    fn good_korean_text_passes_sanity() {
        assert!(looks_like_korean_text("안녕하세요 정상 텍스트입니다"));
        // 치환문자 가득이면 깨짐으로 판정.
        assert!(!looks_like_korean_text(
            "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}내용"
        ));
        assert!(!looks_like_korean_text(""));
    }

    #[test]
    fn emoji_lossy_to_cp949() {
        // CP949에 없는 글자(이모지)는 손실 신호.
        let (_, lossy) = utf8_to_cp949("웃음 😀");
        assert!(lossy);
    }
}

#[cfg(test)]
mod image_tests {
    use super::plan_resize;

    #[test]
    fn target_width_keeps_aspect_ratio() {
        // 4000×3000 → 폭 1280이면 높이는 960.
        assert_eq!(plan_resize(4000, 3000, Some(1280), None), (1280, 960));
    }

    #[test]
    fn never_enlarges() {
        // 목표 폭이 원본보다 크면 그대로(확대 안 함).
        assert_eq!(plan_resize(800, 600, Some(2000), None), (800, 600));
        // 배율 1 이상도 그대로.
        assert_eq!(plan_resize(800, 600, None, Some(1.5)), (800, 600));
    }

    #[test]
    fn scale_halves_dimensions() {
        assert_eq!(plan_resize(1000, 800, None, Some(0.5)), (500, 400));
    }

    #[test]
    fn no_options_returns_original() {
        assert_eq!(plan_resize(640, 480, None, None), (640, 480));
        // 잘못된 값은 안전하게 원본.
        assert_eq!(plan_resize(640, 480, Some(0), None), (640, 480));
        assert_eq!(plan_resize(640, 480, None, Some(0.0)), (640, 480));
        // NaN·무한대(잘못된 --배율)도 1×1로 망가지지 않고 원본 유지.
        assert_eq!(plan_resize(640, 480, None, Some(f64::NAN)), (640, 480));
        assert_eq!(plan_resize(640, 480, None, Some(f64::INFINITY)), (640, 480));
    }
}

#[cfg(test)]
mod zipview_tests {
    use crate::archive::decode_zip_name;

    #[test]
    fn utf8_name_passthrough() {
        assert_eq!(decode_zip_name("한글.txt".as_bytes()), "한글.txt");
        assert_eq!(decode_zip_name(b"report.pdf"), "report.pdf");
    }

    #[test]
    fn cp949_name_decoded() {
        // 윈도우 zip의 CP949 파일명을 복구.
        let cp949 = encoding_rs::EUC_KR.encode("계약서.hwp").0.into_owned();
        // CP949 바이트는 보통 유효한 UTF-8이 아니므로 EUC-KR로 디코드돼야 한다.
        assert_eq!(decode_zip_name(&cp949), "계약서.hwp");
    }
}

#[cfg(test)]
mod flatten_tests {
    use super::flatten_on_white;

    #[test]
    fn transparent_becomes_white_opaque_preserved() {
        let mut rgba = image::RgbaImage::new(3, 1);
        rgba.put_pixel(0, 0, image::Rgba([255, 0, 0, 255])); // 불투명 빨강 → 그대로
        rgba.put_pixel(1, 0, image::Rgba([0, 0, 0, 0])); // 완전 투명 → 흰색
        rgba.put_pixel(2, 0, image::Rgba([0, 0, 0, 128])); // 반투명 검정 → 회색
        let dynimg = image::DynamicImage::ImageRgba8(rgba);
        let out = flatten_on_white(&dynimg);
        assert_eq!(out.get_pixel(0, 0).0, [255, 0, 0]);
        assert_eq!(out.get_pixel(1, 0).0, [255, 255, 255]);
        // 반투명 검정(a=128)을 흰 위에: 약 127.
        let g = out.get_pixel(2, 0).0;
        assert!(g[0] >= 125 && g[0] <= 129, "회색 기대, got {g:?}");
    }
}

#[cfg(test)]
mod alert_state_tests {
    use super::AlertState;

    #[test]
    fn roundtrips_and_defaults_missing_fields() {
        let s = AlertState {
            last_briefed: Some("2026-06-03".into()),
            last_holiday: None,
            last_habit: Some("2026-06-03".into()),
            last_weekly: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: AlertState = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
        // 빈 JSON·누락 필드는 기본값(None)으로 복원 — 구버전 파일 호환.
        let partial: AlertState = serde_json::from_str("{}").unwrap();
        assert_eq!(partial, AlertState::default());
    }
}

#[cfg(test)]
mod bare_preset_tests {
    use super::resolve_bare_preset;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn bare_preset_name_resolves() {
        // `wonjang 일지` → 일지 프리셋 프롬프트(옵시디언 일지 기록).
        let (full, note) = resolve_bare_preset(&s(&["일지"])).expect("일지 프리셋");
        assert!(full.contains("옵시디언"), "프리셋 프롬프트여야: {full}");
        assert!(note.contains("일지"));
    }

    #[test]
    fn extra_words_become_instruction() {
        let (full, _) = resolve_bare_preset(&s(&["일지", "오늘", "운동함"])).expect("일지");
        assert!(full.contains("추가 지시: 오늘 운동함"), "{full}");
    }

    #[test]
    fn alias_resolves() {
        // 메모의 별칭 'memo'.
        assert!(resolve_bare_preset(&s(&["memo"])).is_some());
    }

    #[test]
    fn non_preset_is_none() {
        // 평범한 자연어 첫 단어는 건드리지 않는다.
        assert!(resolve_bare_preset(&s(&["안녕하세요", "오늘", "날씨"])).is_none());
        assert!(resolve_bare_preset(&[]).is_none());
        // 따옴표로 묶인 한 덩어리는 정확히 일치하지 않아 자연어로.
        assert!(resolve_bare_preset(&s(&["일지 써줘"])).is_none());
    }
}

#[cfg(test)]
mod export_path_tests {
    use super::pick_export_path;
    use std::path::PathBuf;

    #[test]
    fn uses_download_dir_when_present() {
        let p = pick_export_path(Some(PathBuf::from("/home/u/Downloads")), "디데이.ics");
        assert_eq!(p, PathBuf::from("/home/u/Downloads/디데이.ics"));
    }

    #[test]
    fn falls_back_to_cwd_when_absent() {
        // 다운로드 폴더가 없으면 종전대로 현재 폴더에 파일 이름만으로.
        let p = pick_export_path(None, "가계부.csv");
        assert_eq!(p, PathBuf::from("가계부.csv"));
    }
}

#[cfg(test)]
mod calc_guard_tests {
    use super::{cmd_salary, cmd_vat, cmd_wage};

    // 0·음수·비유한 입력은 "0원" 같은 무의미 출력 대신 정직하게 거부한다(v1.23.3 원칙 확장).
    #[test]
    fn salary_rejects_meaningless_input() {
        assert!(cmd_salary(0.0).is_err());
        assert!(cmd_salary(-100.0).is_err());
        assert!(cmd_salary(f64::INFINITY).is_err());
        assert!(cmd_salary(f64::NAN).is_err());
        assert!(cmd_salary(3600.0).is_ok()); // 정상은 그대로.
    }

    #[test]
    fn vat_rejects_meaningless_input() {
        assert!(cmd_vat(0.0).is_err());
        assert!(cmd_vat(-1.0).is_err());
        assert!(cmd_vat(f64::INFINITY).is_err());
        assert!(cmd_vat(11_000.0).is_ok());
    }

    #[test]
    fn wage_rejects_meaningless_input() {
        assert!(cmd_wage(10_030.0, 0.0).is_err()); // 0시간.
        assert!(cmd_wage(0.0, 40.0).is_err()); // 0시급.
        assert!(cmd_wage(-1.0, 40.0).is_err());
        assert!(cmd_wage(f64::NAN, 40.0).is_err());
        assert!(cmd_wage(10_030.0, 40.0).is_ok());
    }

    #[test]
    fn char_limit_note_boundaries() {
        use super::char_limit_note;
        assert_eq!(
            char_limit_note(936, Some(1000)),
            "  (제한 1000 → 64자 남음)"
        );
        // 딱 제한이면 초과가 아니라 0자 남음.
        assert_eq!(
            char_limit_note(1000, Some(1000)),
            "  (제한 1000 → 0자 남음)"
        );
        assert_eq!(
            char_limit_note(1050, Some(1000)),
            "  (제한 1000 → 50자 초과 ⚠️)"
        );
        assert_eq!(char_limit_note(50, None), "");
    }
}

#[cfg(test)]
mod alias_tests {
    use super::{Cli, Commands};
    use clap::Parser;

    fn cmd_of(args: &[&str]) -> Commands {
        // clap-derive가 만드는 command() 빌더는 서브커맨드가 늘수록 디버그 스택 프레임이
        // 커져 기본 2MB 테스트 스레드 스택을 넘긴다(릴리스 바이너리 메인 스레드 8MB는 정상).
        // 바이너리와 같은 크기의 스택에서 파싱한다.
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                Cli::try_parse_from(&owned)
                    .expect("파싱 성공")
                    .command
                    .expect("서브커맨드")
            })
            .expect("스레드 생성")
            .join()
            .expect("파싱 스레드")
    }

    // 한국인이 가장 자연스럽게 떠올리는 단어가 곧장 해당 기능으로 가야 한다.
    // 별칭이 없으면 그 단어는 AI로 위임돼(느리고 예측 불가, 미연결 시 실패) 기능을 놓친다.
    #[test]
    fn loanword_aliases_resolve_to_local_commands() {
        assert!(matches!(
            cmd_of(&["wonjang", "북마크"]),
            Commands::Bookmark { .. }
        ));
        assert!(matches!(
            cmd_of(&["wonjang", "즐찾"]),
            Commands::Bookmark { .. }
        ));
        assert!(matches!(
            cmd_of(&["wonjang", "타이머"]),
            Commands::Focus { .. }
        ));
        assert!(matches!(
            cmd_of(&["wonjang", "리마인더"]),
            Commands::Remind { .. }
        ));
        assert!(matches!(
            cmd_of(&["wonjang", "투두"]),
            Commands::Todo { .. }
        ));
    }

    // 기존 별칭도 그대로 유지(회귀 방지).
    #[test]
    fn existing_aliases_still_work() {
        assert!(matches!(
            cmd_of(&["wonjang", "즐겨찾기"]),
            Commands::Bookmark { .. }
        ));
        assert!(matches!(
            cmd_of(&["wonjang", "집중"]),
            Commands::Focus { .. }
        ));
        assert!(matches!(
            cmd_of(&["wonjang", "약속"]),
            Commands::Remind { .. }
        ));
        assert!(matches!(
            cmd_of(&["wonjang", "할일"]),
            Commands::Todo { .. }
        ));
    }
}

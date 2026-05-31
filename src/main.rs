//! 원장 에이전트 — 로컬 환경을 다루는 한국어 우선 자율 AI 에이전트 (Rust).
//!
//! 헤르메스 에이전트(NousResearch/hermes-agent)의 핵심 아이디어를 러스트로
//! 재구성한다: 제공자 무관 LLM, 로컬 도구, 에이전트 루프, 한국어 우선 UX.

mod agent;
mod cli_backend;
mod clipboard;
mod config;
mod cron;
mod ddays;
mod engine;
mod expenses;
mod focus;
mod gateway;
mod habits;
mod llm;
mod mcp;
mod memory;
mod notes;
mod notion;
mod preset;
mod push;
mod reminders;
mod safety;
mod session;
mod skill;
mod todos;
mod tools;
mod ui;
mod util;
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
    /// 비서 현황을 한눈에 봅니다(약속·할일·디데이·예약작업).
    #[command(alias = "현황")]
    Status,
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
        Some(Commands::Status) => return cmd_status(),
        Some(Commands::Expense { action }) => return cmd_expense(action),
        Some(Commands::Habit { action }) => return cmd_habit(action),
        Some(Commands::Focus { minutes, label }) => return cmd_focus(*minutes, label),
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
        "  텔레그램  : {} / 허용 chat_id {}개",
        if cfg.telegram_token.is_empty() {
            "토큰 없음"
        } else {
            "토큰 설정됨"
        },
        cfg.telegram_allowed_ids.len()
    );
    ui::info("\nAPI 키·토큰 등 비밀값은 파일에 저장하지 않습니다. 환경 변수를 사용하세요.");
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
    // 무인 실행이지만 위험 명령은 기본 차단(allow_dangerous=false).
    let ctx = ToolContext {
        auto_approve: true,
        allow_dangerous: false,
    };
    let tick = std::time::Duration::from_secs(30);

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

        tokio::time::sleep(tick).await;
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

/// 비서 현황 대시보드(약속·할일·디데이·예약작업) — LLM 없이 즉시.
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

    // 예약 작업.
    let cron = cron::CronStore::load()?;
    let enabled = cron.tasks.iter().filter(|t| t.enabled).count();
    if enabled > 0 {
        println!("  🔁 예약 작업 {enabled}개 등록됨");
    }

    println!();
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

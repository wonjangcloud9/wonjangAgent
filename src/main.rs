//! 원장 에이전트 — 로컬 환경을 다루는 한국어 우선 자율 AI 에이전트 (Rust).
//!
//! 헤르메스 에이전트(NousResearch/hermes-agent)의 핵심 아이디어를 러스트로
//! 재구성한다: 제공자 무관 LLM, 로컬 도구, 에이전트 루프, 한국어 우선 UX.

mod agent;
mod config;
mod cron;
mod llm;
mod memory;
mod session;
mod skill;
mod tools;
mod ui;
mod web;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
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
    /// 예약 작업(크론)을 관리하고 실행합니다.
    Cron {
        #[command(subcommand)]
        action: CronAction,
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
        Some(Commands::Cron { action }) => match action {
            CronAction::Add { schedule, prompt } => return cmd_cron_add(schedule, prompt),
            CronAction::List => return cmd_cron_list(),
            CronAction::Remove { id } => return cmd_cron_remove(*id),
            CronAction::Run => {} // 아래에서 클라이언트 구성 후 데몬 실행.
        },
        None => {}
    }

    // API 키 확인.
    if cfg.api_key.is_empty() {
        ui::error("API 키가 없습니다.");
        ui::info(
            "환경 변수로 키를 설정해 주세요. 예시:\n  \
             export OPENROUTER_API_KEY=sk-...\n  \
             export WONJANG_MODEL=anthropic/claude-3.5-sonnet\n\
             자세한 설정은 `wonjang config` 를 실행하세요.",
        );
        std::process::exit(1);
    }

    let client = LlmClient::new(cfg.base_url.clone(), cfg.api_key.clone(), cfg.model.clone());
    let tools = default_tools();
    let ctx = ToolContext {
        auto_approve: cli.yes,
    };

    // 크론 데몬(LLM 필요).
    if let Some(Commands::Cron {
        action: CronAction::Run,
    }) = &cli.command
    {
        return cmd_cron_run(&client, &cfg, &tools).await;
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

    // 단발 실행 모드.
    let one_shot = cli.prompt.join(" ");
    if !one_shot.trim().is_empty() {
        messages.push(Message::user(one_shot));
        agent::run_turn(&client, &cfg, &tools, &ctx, &mut messages).await?;
        sess.save(&messages).ok();
        return Ok(());
    }

    // 대화형 REPL 모드.
    repl(&client, &cfg, &tools, &ctx, &mut messages, &sess).await
}

/// 대화형 모드.
async fn repl(
    client: &LlmClient,
    cfg: &Config,
    tools: &[Box<dyn tools::Tool>],
    ctx: &ToolContext,
    messages: &mut Vec<Message>,
    sess: &session::Session,
) -> Result<()> {
    ui::banner(&cfg.model);

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
        if let Err(e) = agent::run_turn(client, cfg, tools, ctx, messages).await {
            ui::error(&format!("{e:#}"));
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
        ui::note(&format!("기본 설정 파일을 생성했습니다: {}", saved.display()));
    }
    println!("{}", "현재 설정:".to_string());
    println!("  설정 파일 : {}", path.display());
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
    ui::info("\nAPI 키는 보안을 위해 파일에 저장하지 않습니다. 환경 변수를 사용하세요.");
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
async fn cmd_cron_run(
    client: &LlmClient,
    cfg: &Config,
    tools: &[Box<dyn tools::Tool>],
) -> Result<()> {
    let store = cron::CronStore::load()?;
    ui::note(&format!(
        "스케줄러 시작 — 등록된 작업 {}개. 종료는 Ctrl-C.",
        store.tasks.len()
    ));
    // 무인 실행이므로 도구를 자동 승인한다.
    let ctx = ToolContext { auto_approve: true };
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
                Message::system(agent::system_prompt(mem.prompt_block(), skills.prompt_block())),
                Message::user(prompt),
            ];
            if let Err(e) = agent::run_turn(client, cfg, tools, &ctx, &mut messages).await {
                ui::error(&format!("작업 #{id} 오류: {e:#}"));
            }

            // 실행 시각 기록.
            if let Some(t) = store.tasks.iter_mut().find(|t| t.id == id) {
                t.last_run_ms = Some(cron::now_ms());
            }
            store.save().ok();
        }

        tokio::time::sleep(tick).await;
    }
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

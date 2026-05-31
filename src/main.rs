//! 원장 에이전트 — 로컬 환경을 다루는 한국어 우선 자율 AI 에이전트 (Rust).
//!
//! 헤르메스 에이전트(NousResearch/hermes-agent)의 핵심 아이디어를 러스트로
//! 재구성한다: 제공자 무관 LLM, 로컬 도구, 에이전트 루프, 한국어 우선 UX.

mod agent;
mod config;
mod llm;
mod tools;
mod ui;

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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 현재 설정을 보여주고, 없으면 기본 설정 파일을 생성합니다.
    Config,
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

    // 서브커맨드 처리.
    if let Some(Commands::Config) = cli.command {
        return cmd_config(&cfg);
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

    let mut messages = vec![Message::system(agent::system_prompt())];

    // 단발 실행 모드.
    let one_shot = cli.prompt.join(" ");
    if !one_shot.trim().is_empty() {
        messages.push(Message::user(one_shot));
        agent::run_turn(&client, &cfg, &tools, &ctx, &mut messages).await?;
        return Ok(());
    }

    // 대화형 REPL 모드.
    repl(&client, &cfg, &tools, &ctx, &mut messages).await
}

/// 대화형 모드.
async fn repl(
    client: &LlmClient,
    cfg: &Config,
    tools: &[Box<dyn tools::Tool>],
    ctx: &ToolContext,
    messages: &mut Vec<Message>,
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
                ui::info("대화 기록을 초기화했습니다.");
                continue;
            }
            _ => {}
        }

        messages.push(Message::user(input.to_string()));
        if let Err(e) = agent::run_turn(client, cfg, tools, ctx, messages).await {
            ui::error(&format!("{e:#}"));
        }
    }
    Ok(())
}

fn print_help() {
    ui::info(
        "사용 가능한 명령:\n  \
         /help     이 도움말\n  \
         /reset    대화 기록 초기화\n  \
         /exit     종료\n\n\
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

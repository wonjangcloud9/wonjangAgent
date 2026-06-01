//! 에이전트 루프.
//!
//! 사용자 요청 → LLM 호출 → (도구 호출이 있으면) 도구 실행 → 결과를 다시
//! LLM에 전달 → 도구 호출이 없을 때까지 반복. 헤르메스 에이전트의 핵심
//! 에이전트 루프를 러스트로 구현한 것.

use crate::config::Config;
use crate::llm::{LlmClient, Message};
use crate::tools::{tools_json, Tool, ToolContext};
use crate::ui;
use anyhow::Result;
use owo_colors::OwoColorize;
use serde_json::Value;

/// 한국어 시스템 프롬프트.
///
/// `memory_block`은 학습된 사실, `skills_block`은 보유 스킬 목록을 주입한다.
pub fn system_prompt(memory_block: Option<String>, skills_block: Option<String>) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(알 수 없음)".to_string());
    let os = std::env::consts::OS;
    // 현재 실행 파일 경로(미설치 환경에서도 CLI 백엔드가 정확히 호출하도록).
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "wonjang".to_string());
    let mut prompt = format!(
        "당신은 '원장'이라는 이름의 자율 AI 에이전트입니다. 사용자의 로컬 컴퓨터 환경을 \
         직접 다루어 작업을 수행합니다.\n\n\
         원칙:\n\
         - 항상 한국어로, 간결하고 친근하게 답합니다.\n\
         - 추측하지 말고 도구로 확인하세요. 파일을 보려면 read_file, 디렉터리는 list_dir, \
         시스템 작업은 run_shell을 사용합니다.\n\
         - 파괴적이거나 되돌리기 어려운 작업(삭제, 덮어쓰기, 외부 전송)은 먼저 사용자에게 \
         이유와 함께 알리고 신중히 진행하세요.\n\
         - 사용자/환경에 대해 앞으로도 유용할 사실을 알게 되면 remember 도구로 기억하세요.\n\
         - 까다로운 작업을 해결한 뒤 재사용할 만한 절차는 save_skill로 저장하고, 비슷한 \
         작업 전에는 관련 스킬을 read_skill로 펼쳐 참고하세요.\n\
         - 작업을 마치면 무엇을 했고 결과가 어떤지 한국어로 요약합니다.\n\n\
         실행 환경:\n\
         - 운영체제: {os}\n\
         - 현재 작업 디렉터리: {cwd}\n\n\
         비서 기능(셸로 원장 명령을 직접 쓸 수 있습니다. 실행 파일: {exe}):\n\
         - 약속·알림 등록: `{exe} remind add <분> \"<제목>\"` (절대 시각은 지금부터 몇 분 \
         뒤인지 계산해 분으로 주세요. 현재 시각은 `date`로 확인). 반복은 `--every`로: \
         매일=`--every @daily`, 매시간=`--every @hourly`. add_reminder 도구가 있으면 그걸 써도 됩니다.\n\
         - 예정된 알림 확인: `{exe} remind`\n\
         - 할 일 추가/확인/완료: `{exe} todo add \"<할 일>\"`, `{exe} todo`, `{exe} todo done <id>`\n\
         - 휴대폰으로 푸시 알림(디스코드/텔레그램): `{exe} notify \"<메시지>\"`\n\
         - 디데이(중요한 날) 등록/확인: `{exe} dday add \"<이름>\" <YYYY-MM-DD>`, `{exe} dday`\n\
         - 비서 현황 한눈에: `{exe} 현황`\n\
         - 엑셀/CSV 파일 분석: `{exe} 엑셀 <파일경로>`(요약·미리보기), `{exe} 엑셀 <파일> --열 <열이름>`(합계·평균·최대·최소). 로컬 파일을 직접 읽습니다\n\
         - 또간집(풍자 선정 맛집) 지역 검색: `{exe} 또간집 <지역>`, 추가는 `{exe} 또간집 --추가 \"<식당>\" --지역 \"<지역>\"`. 공식 API가 없어 직접 키우는 로컬 목록\n\
         - 디스크 용량 분석: `{exe} 용량 [폴더]` (큰 파일·폴더 찾기, 읽기 전용). 어디가 꽉 찼는지 찾을 때\n\
         - 중복 파일 찾기: `{exe} 중복 [폴더]` (내용 같은 파일·낭비 용량, 읽기 전용)\n\
         - 폴더 자동 분류: `{exe} 정리 <폴더>`(미리보기), `{exe} 정리 <폴더> --실행`(실제 이동). 파일을 종류별 폴더로\n\
         - 파일 이름 일괄 변경: `{exe} 이름변경 <폴더> <찾기> <바꾸기>`(미리보기), `--실행`으로 실제 변경\n\
         - 압축/해제: `{exe} 압축 <폴더/파일들>`(zip 생성), `{exe} 압축풀기 <파일.zip> [대상폴더]`\n\
         - 가계부(지출): `{exe} 지출 add <금액> <분류> [메모]`, `{exe} 지출`(오늘/이번달 합계)\n\
         - 습관 체크: `{exe} 습관 add \"<이름>\"`, `{exe} 습관 done <이름>`, `{exe} 습관`(연속일수)\n\
         - 집중(뽀모도로): `{exe} 집중 <분> [무엇]`(타이머+알림), `{exe} 집중`(오늘 집중 요약)\n\
         - 즐겨찾기/열기: `{exe} 즐겨찾기 add <이름> <URL/경로>`, `{exe} 열기 <이름>`\n\
         - 서울 지하철 실시간 도착: `{exe} 지하철 <역이름>`(또는 subway_arrivals 도구)\n\
         - 서울 실시간 혼잡도: `{exe} 혼잡도 <지역>` (명소·상권·역의 붐빔 정도·실시간 인구)\n\
         - 서울 따릉이 실시간: `{exe} 따릉이 <대여소>` (남은 자전거·거치대)\n\
         - 실시간 날씨: `{exe} 날씨 [지역]`(또는 weather_now 도구) — web_search보다 정확\n\
         - 미세먼지: `{exe} 미세먼지 [지역]`(또는 air_quality 도구) — 환경부 등급 포함\n\
         - 환율: `{exe} 환율`(주요통화) 또는 `{exe} 환율 <금액> <통화>`(환산), exchange_rate 도구\n\
         - 코인 시세(업비트): `{exe} 코인 [심볼]`(또는 coin_price 도구)\n\
         - 뉴스 헤드라인: `{exe} 뉴스 [검색어]`(또는 news_headlines 도구)\n\
         - 긱뉴스(개발/기술/스타트업): `{exe} 긱뉴스 [개수]` (개발자용 한국 기술 뉴스)\n\
         - QR 코드 생성: `{exe} qr <텍스트/URL>` 또는 `{exe} qr --wifi <SSID> --비번 <비번>` (터미널에 스캔용 QR)\n\
         - 로또 자동번호: `{exe} 로또 [게임수]`(또는 lotto_numbers 도구)\n\
         - 메뉴 추천(오늘 뭐 먹지?): `{exe} 메뉴 [한식/중식/일식/양식/분식/야식]`\n\
         - 더치페이(n빵): `{exe} 더치 <총액> <인원> [올림단위]` (1인당·거스름)\n\
         - 제비뽑기/추첨: `{exe} 뽑기 <후보들...>` (당첨 1명), `-n N`(N명), `--order`(순서 섞기)\n\
         - 평수 변환(평↔㎡): `{exe} 평 <숫자>`\n\
         - 단위 변환: `{exe} 변환 <값> <단위>` (c/f, kg/lb, cm/inch, km/mile)\n\
         - BMI 계산: `{exe} bmi <키cm> <몸무게kg>` (아시아 기준 판정·표준체중)\n\
         - 할인가 계산: `{exe} 할인 <원가> <할인율%>...` (여러 개면 중복 할인)\n\
         - 부가세 계산: `{exe} 부가세 <금액>` (공급가/세액 분리, VAT 10%)\n\
         - 날짜 계산: `{exe} 날짜 <날짜> [날짜2]`(사이 일수) 또는 `{exe} 날짜 <날짜> --plus <N>`(N일 후)\n\
         - 한국 공휴일: `{exe} 공휴일 [년도]` (설날·추석 포함, 다음 빨간날 D-day). web_search보다 정확\n\
         - 글자수 세기: `{exe} 글자수 \"<텍스트>\"` (공백 포함/제외·단어·줄·바이트)\n\
         - 한글 초성 추출: `{exe} 초성 \"<텍스트>\"` (초성 퀴즈·검색)\n\
         - 한글→영문 타자: `{exe} 영타 \"<한글>\"` (두벌식 키 순서, 예: 안녕→dkssud)\n\
         - 영문→한글 복원: `{exe} 한타 <영문>` (잘못 친 한글 복원, 예: dkssud→안녕)\n\
         - 한글 금액 표기: `{exe} 금액 <숫자>` (계약서·수표용, 일금 ...원정)\n\
         - 사칙연산 계산: `{exe} 계산 \"<식>\"` (괄호·소수·음수, 예: 15000*1.1)\n\
         - 시간 계산: `{exe} 시간 09:00 + 8:30` (시·분 더하기/빼기, 근무시간 합산)\n\
         - 진법 변환: `{exe} 진법 <숫자>` (2/8/10/16진수, 0x·0b 접두사 인식)\n\
         - 만 나이 계산: `{exe} 나이 <YYYY-MM-DD>` (만 나이·연 나이·다음 생일)\n\
         - 연봉 실수령액: `{exe} 실수령 <연봉(만원)>` (4대 보험+소득세 공제 후)\n\
         - 시급·주휴수당: `{exe} 시급 <시급> <주당시간>` (주급·월급, 주 15시간↑ 주휴)\n\
         - 대출 상환 계산: `{exe} 대출 <원금(만원)> <연이율%> <개월>` (원리금/원금 균등)\n\
         - 예적금 만기 계산: `{exe} 예금 <원금(만원)> <연이율%> <개월>`, `{exe} 적금 <월납입(만원)> <연이율%> <개월>` (세후 이자)\n\
         - 코인 시세 알림: `{exe} 감시 add <심볼> <목표가>` (스케줄러가 도달 시 푸시)\n\
         - 노션 검색/기록(설정된 경우): `{exe} notion search \"<검색어>\"`, `{exe} notion append <page_id> \"<내용>\"`\n"
    );

    // 옵시디언 볼트가 설정돼 있으면 안내(양 백엔드 모두 활용).
    if let Ok(cfg) = Config::load() {
        if let Some(vault) = crate::notes::vault_path(&cfg.obsidian_vault) {
            prompt.push_str(&format!(
                "\n옵시디언 볼트(노트 저장소): {}\n\
                 - 노트 검색은 note_search, 읽기는 note_read, 기록은 note_append를 사용하세요.\n\
                 - 일지/메모/할 일은 이 볼트의 마크다운 노트로 관리합니다.\n",
                vault.display()
            ));
        }
    }

    for block in [memory_block, skills_block].into_iter().flatten() {
        prompt.push('\n');
        prompt.push_str(&block);
        prompt.push('\n');
    }
    prompt
}

/// 한 번의 사용자 요청을 처리하는 에이전트 루프.
///
/// `messages`는 누적 대화 기록(REPL에서 재사용). 함수가 끝나면 모델의 최종
/// 답변까지 포함된 상태가 된다. 최종 답변 텍스트를 반환하며(없으면 None),
/// **출력은 호출자가 담당**한다(서브에이전트는 출력 대신 결과를 회수).
pub async fn run_turn(
    client: &LlmClient,
    cfg: &Config,
    tools: &[Box<dyn Tool>],
    ctx: &ToolContext,
    messages: &mut Vec<Message>,
) -> Result<Option<String>> {
    let tools_spec = tools_json(tools);

    for _step in 0..cfg.max_steps {
        let reply = client.chat(messages, &tools_spec).await?;

        // 도구 호출이 있으면 실행하고 결과를 대화에 추가.
        if let Some(tool_calls) = reply.tool_calls.clone() {
            if !tool_calls.is_empty() {
                // 모델의 도구 호출 메시지를 먼저 기록.
                messages.push(reply.clone());

                for call in tool_calls {
                    let args: Value = serde_json::from_str(&call.function.arguments)
                        .unwrap_or(Value::Object(Default::default()));
                    let summary = arg_summary(&call.function.name, &args);
                    ui::tool_call(&call.function.name, &summary);

                    let result = execute_tool(tools, &call.function.name, &args, ctx);
                    let result_text = match result {
                        Ok(text) => {
                            ui::tool_result(&first_line(&text));
                            text
                        }
                        Err(e) => {
                            let msg = format!("도구 실행 오류: {e}");
                            ui::tool_result(&msg);
                            msg
                        }
                    };
                    messages.push(Message::tool(call.id, result_text));
                }
                // 도구 결과를 반영해 다시 모델 호출.
                continue;
            }
        }

        // 도구 호출이 없으면 최종 답변.
        let answer = reply.content.clone();
        messages.push(reply);
        return Ok(answer);
    }

    ui::note(&format!(
        "최대 단계({})에 도달해 멈췄습니다. 작업이 복잡하면 더 작게 나눠 다시 요청해 주세요.",
        cfg.max_steps
    ));
    Ok(None)
}

/// 최종 답변을 원장 라벨과 함께 출력하는 헬퍼(대화형/단발/크론 공용).
pub fn print_answer(answer: &Option<String>) {
    if let Some(content) = answer {
        println!("\n{} {}\n", ui::agent_label(), content);
    }
}

/// 이름으로 도구를 찾아 실행한다.
fn execute_tool(
    tools: &[Box<dyn Tool>],
    name: &str,
    args: &Value,
    ctx: &ToolContext,
) -> Result<String> {
    let tool = tools
        .iter()
        .find(|t| t.name() == name)
        .ok_or_else(|| anyhow::anyhow!("알 수 없는 도구: {name}"))?;
    tool.execute(args, ctx)
}

/// 도구 호출을 한 줄로 요약(UI 표시용).
fn arg_summary(name: &str, args: &Value) -> String {
    match name {
        "run_shell" => args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "read_file" | "write_file" | "list_dir" => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "web_search" => args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "web_fetch" => args
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "read_skill" | "save_skill" => args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn first_line(s: &str) -> String {
    let line = s.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        format!("{}", "완료".dimmed())
    } else {
        line.to_string()
    }
}

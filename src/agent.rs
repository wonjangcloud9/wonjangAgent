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
    // 페르소나(성격)를 맨 앞에 — 사용자가 SOUL.md로 소유한다(헤르메스 방식).
    let persona = crate::soul::active_persona();
    let mut prompt = format!(
        "{persona}\n\n\
         당신은 사용자의 로컬 컴퓨터 환경을 직접 다루어 작업을 수행하는 자율 AI 에이전트입니다.\n\n\
         원칙:\n\
         - 위에 정해진 성격·말투를 모든 답변에서 일관되게 유지하세요.\n\
         - 항상 한국어로 답합니다.\n\
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
         - 휴대폰으로 푸시 알림(카카오/디스코드/슬랙/텔레그램): `{exe} notify \"<메시지>\"`. \"카카오로 보내줘\"처럼 채널을 짚어도 같은 명령이면 됩니다(설정된 채널로 모두 전송)\n\
         - 디데이(중요한 날) 등록/확인: `{exe} dday add \"<이름>\" <YYYY-MM-DD>`, `{exe} dday`. **공유 카드**: `{exe} 디데이 카드 [이름]`(수능 D-100 같은 카운트다운을 카톡에 안 깨지는 한 장 카드로, `--복사`로 클립보드·`--폭 34` 카톡). 캘린더로 내보내기: `{exe} 디데이 내보내기`(.ics). \"수능 디데이 카드 만들어줘\"·\"디데이 캘린더에 넣게 내보내줘\"\n\
         - 비서 현황 한눈에: `{exe} 현황`\n\
         - 자랑 카드(회고): `{exe} 자랑`(이번 달 습관 잔디·집중·지출·D-day를 카톡에 안 깨지는 한 장 카드로), `{exe} 자랑 --주`(이번 주 + **지난주 대비 ▲▼**). 단톡방·SNS에 캡처해 자랑하기 좋음. 카톡은 `--폭 34`, 바로 복사는 `--복사`(붙여넣기만). \"이번 달 자랑 카드\"/\"이번 주 결산\"/\"카드 복사해줘\"\n\
         - 엑셀/CSV 분석: `{exe} 엑셀 <파일>`(요약·미리보기), `--열 <열>`(합계·평균·최대·최소), **`--그룹 <분류열> --열 <숫자열>`(분류별 집계=피벗+막대그래프)**, **`--필터 지역=서울`(조건 행만; `=`,`!=`,`>`,`<`,`>=`,`<=`,`~`포함; 정렬·집계와 조합)**, **`--정렬 <열>`(상위 행, `--오름차순`)**, **`--저장 <out.csv>`(거른·정렬·집계한 결과를 새 CSV로)**, `--시트 <이름>`(다중 시트 엑셀), `--json`. CP949 자동. \"지점별 매출 합계\"·\"서울 것만\"·\"매출 큰 순\"·\"결과 저장\"\n\
         - 여러 표 합치기: `{exe} 표합치기 1월.csv 2월.csv … --저장 합본.csv`(머리글 기준, 열 순서 달라도 이름으로 맞춤, `--출처`로 출처 열). 월별·지점별 파일을 한 장으로 → 바로 `엑셀`로 분석. \"파일들 합쳐서 분석해줘\"\n\
         - 또간집(풍자 선정 맛집) 지역 검색: `{exe} 또간집 <지역>`, 추가는 `{exe} 또간집 --추가 \"<식당>\" --지역 \"<지역>\"`. 공식 API가 없어 직접 키우는 로컬 목록\n\
         - 디스크 용량 분석: `{exe} 용량 [폴더]` (큰 파일·폴더 찾기, 읽기 전용). 어디가 꽉 찼는지 찾을 때\n\
         - 중복 파일 찾기: `{exe} 중복 [폴더]` (내용 같은 파일·낭비 용량, 읽기 전용)\n\
         - 폴더 자동 분류: `{exe} 정리 <폴더>`(미리보기), `{exe} 정리 <폴더> --실행`(실제 이동). 파일을 종류별 폴더로\n\
         - 파일 이름 일괄 변경: `{exe} 이름변경 <폴더> <찾기> <바꾸기>`(미리보기), `--실행`으로 실제 변경\n\
         - 압축/해제: `{exe} 압축 <폴더/파일들>`(zip 생성), `{exe} 압축풀기 <파일.zip> [대상폴더]`, `{exe} 압축보기 <파일.zip>`(풀지 않고 목록만, 한글 파일명 깨짐 보정)\n\
         - 파일 내용 검색: `{exe} 찾기 <폴더> <단어>` (텍스트 파일에서 단어 든 줄 찾기, 읽기 전용)\n\
         - JSON 검증·정렬·값 추출: `{exe} json <파일>` 또는 `{exe} json <파일> --키 <점경로>`\n\
         - 파일 체크섬: `{exe} 해시 <파일>` (SHA-256, `--확인 <값>`으로 무결성 검증)\n\
         - 두 파일 비교: `{exe} 비교 <파일1> <파일2>` (줄 단위 diff, 추가/삭제)\n\
         - 이미지 축소·압축·형식변환: `{exe} 이미지 <사진들> --폭 1280`(여러 장, 첨부 용량↓, EXIF 방향 자동 보정), `--형식 jpg`/`--형식 png`(png↔jpg 변환). 원본 보존. \"사진들 용량 줄여줘\"/\"png를 jpg로 바꿔줘\"\n\
         - 여러 사진 → PDF 한 파일: `{exe} 사진묶기 <사진1> <사진2> …`(서류 제출·스캔앱 대용). \"사진들 PDF로 묶어줘\"\n\
         - 여러 이미지 → 한 장으로 이어붙이기: `{exe} 이미지이어붙이기 <1> <2> … --세로`(기본; `--가로`도). 긴 카톡 캡처·영수증 여러 장을 한 이미지로. \"캡처들 하나로 이어줘\"\n\
         - PDF 합치기: `{exe} pdf합치기 <pdf1> <pdf2> …`(서류 합본). \"PDF들 하나로 합쳐줘\"\n\
         - PDF 페이지 추출: `{exe} pdf페이지 <파일> 1-3,5`(원하는 페이지만 새 PDF로). \"이 PDF 1~3쪽만\"\n\
         - PDF 회전: `{exe} pdf회전 <파일> 90`(옆으로 스캔된 서류 바로 세우기, 90의 배수). \"PDF 돌려줘\"\n\
         - PDF 비밀번호 걸기: `{exe} pdf암호 <파일> --비번 <비밀번호>`(AES-256, 민감 서류 제출용). 원본은 보존하고 `_암호.pdf`로 저장. ⚠️ 사용자에게 비밀번호를 정확히 확인받고, 잊으면 못 연다는 점을 알리세요. \"이 PDF에 비밀번호 걸어줘\"\n\
         - 한글 깨진 파일 복구: `{exe} 깨짐 <파일.csv>`(EUC-KR/CP949 → UTF-8). \"엑셀/메모장 한글 깨져\"\n\
         - 받은편지함 읽기(IMAP): `{exe} 메일`(최근 목록), `{exe} 메일 --안읽음`(안 읽은 것만), `{exe} 메일읽기 <번호>`(그 메일 본문 읽기, 1=최신). 환경변수 WONJANG_EMAIL·WONJANG_EMAIL_PASSWORD(앱 비밀번호) 필요 — 미설정이면 명령이 설정법을 안내함. \"메일 왔어?\"/\"안 읽은 메일\"/\"첫 메일 내용 읽어줘\"\n\
         - 메일 검색: `{exe} 메일검색 <키워드>`(보낸이·제목에서 찾기, 최근 100통 기본 — `--최근 300`으로 범위↑). \"영수증 메일 찾아줘\"/\"김부장한테 온 메일\"\n\
         - 메일 첨부 저장: `{exe} 메일첨부 <번호> --저장폴더 <경로>`(그 메일의 첨부파일을 폴더에 저장, 경로 생략 시 현재 폴더). 저장 뒤 `{exe} 엑셀`/`{exe} pdf페이지` 등으로 바로 이어서 처리 가능. \"그 메일 첨부 받아줘\"\n\
         - 메일 보내기(SMTP, 파일 첨부 가능): `{exe} 메일보내기 --받는사람 a@b.com --제목 \"...\" --내용 \"...\" [--첨부 파일경로 --첨부 다른파일]`. ⚠️ 외부 전송(되돌릴 수 없음)이니 **보내기 전 반드시 받는사람·제목·내용·첨부를 사용자에게 보여주고 동의를 받은 뒤** 실행하세요. \"이 PDF 메일로 보내줘\"\n\
         - 가계부(지출): `{exe} 지출 add <금액> <분류> [메모]`, `{exe} 지출`(오늘/이번달 합계). CSV로 내보내 분석: `{exe} 지출 내보내기` → `{exe} 엑셀 가계부.csv --그룹 분류 --열 금액`(월별·분류별 추이). \"가계부 분석해줘\"\n\
         - 간단 일기: `{exe} 일기 \"<내용>\"`(기록), `{exe} 일기`(이번 달 보기) — 옵시디언 없이도\n\
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
         - 세계 시간(주요 도시, DST 정확): `{exe} 세계시간 [도시]`\n\
         - 시간대 변환: `{exe} 시차 <HH:MM> <출발도시> <도착도시>` (해외 회의 시각 환산)\n\
         - 뉴스 헤드라인: `{exe} 뉴스 [검색어]`(또는 news_headlines 도구)\n\
         - 긱뉴스(개발/기술/스타트업): `{exe} 긱뉴스 [개수]` (개발자용 한국 기술 뉴스)\n\
         - GitHub 저장소 정보: `{exe} 깃헙 <owner/repo>` (별·이슈·최신 릴리스)\n\
         - 내 공인 IP·통신사·위치: `{exe} 내아이피` (VPN/네트워크 확인)\n\
         - 사이트/서버 상태 확인: `{exe} 사이트 <url>` (살아있는지·응답속도)\n\
         - QR 코드 생성: `{exe} qr <텍스트/URL>` 또는 `{exe} qr --wifi <SSID> --비번 <비번>` (터미널에 스캔용 QR)\n\
         - 한국 공휴일: `{exe} 공휴일 [년도]` (설날·추석 포함, 다음 빨간날 D-day, **다음 연휴 며칠 연속** + **연차 하나로 만드는 황금연휴** 추천까지 — 대체공휴일·주말 끼임 정확 계산). \"이번 추석 며칠 쉬어\"·\"연차 어떻게 쓰면 황금연휴\"·\"다음 연휴 언제\". web_search보다 정확\n\
         - 생활·금융 계산기(정확한 사용법은 `{exe} 도움` 참고): 평수 `{exe} 평`, 만나이 `{exe} 나이`, 실수령 `{exe} 실수령`, 시급 `{exe} 시급`, 대출 `{exe} 대출`, 예적금 `{exe} 예금`/`{exe} 적금`, 전월세전환(전세→반월세 월세, \"전세 3억 월세로 돌리면\"→`{exe} 전월세 30000 5.5 [보증금만원]`) `{exe} 전월세`, 퇴직금(법정 퇴직금 추정, \"3년 일하고 월급 300이면 퇴직금\"→`{exe} 퇴직금 300 3 [개월]`) `{exe} 퇴직금`, 연차(근로기준법 연차휴가 일수, \"5년차 연차 며칠\"→`{exe} 연차 5`) `{exe} 연차`, 자동차세(비영업 승용, \"내 차 1998cc 자동차세\"→`{exe} 자동차세 1998 [차령]`, 지방교육세·차령경감 포함) `{exe} 자동차세`, 야근수당(연장·야간·휴일 가산, \"시급 12000에 야근 3시간 수당\"→`{exe} 야근수당 12000 --연장 3 [--야간 N] [--휴일 N]`) `{exe} 야근수당`, 할인 `{exe} 할인`, 부가세 `{exe} 부가세`, 날짜·기념일(\"사귄 지 며칠\"·\"100일 언제\"→`{exe} 날짜 <사귄날>`, 며칠째+다음 기념일 양력날짜) `{exe} 날짜`, 시간 `{exe} 시간`, 타임스탬프 `{exe} 타임스탬프`, 진법 `{exe} 진법`, 로마숫자 `{exe} 로마`, 인코딩(base64/url) `{exe} 인코딩`, 색상변환 `{exe} 색`, 단위변환(온도/무게/길이/속도/부피/넓이 + **한국 전통단위 돈·근·관·되·말**, 예: \"금 5돈 몇 g\"→`{exe} 변환 5 돈`, \"고기 두 근\"→`{exe} 변환 2 근`) `{exe} 변환`, BMI `{exe} bmi`, 칼로리(기초대사량) `{exe} 칼로리`, 수면시각(90분주기) `{exe} 수면`, 글자수(자소서 제한 체크 \"1000자 중 몇 자\"→`{exe} 글자수 \"...\" --제한 1000`, 공백 포함/제외 둘 다 남은 글자) `{exe} 글자수`, 초성 `{exe} 초성`, 한영타 `{exe} 영타`/`{exe} 한타`, 한글금액 `{exe} 금액`, 사칙연산 `{exe} 계산`, 더치페이 `{exe} 더치`, 제비뽑기 `{exe} 뽑기`, 비밀번호생성 `{exe} 비번`, UUID `{exe} uuid`, 메뉴추천 `{exe} 메뉴`, 로또 `{exe} 로또`\n\
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

#[cfg(test)]
mod prompt_tests {
    use super::system_prompt;

    #[test]
    fn advertises_recent_file_and_pdf_commands() {
        // 대화형 원장이 최근 추가된 로컬 파일/PDF 능력을 알고 있어야 한다.
        let p = system_prompt(None, None);
        for cmd in [
            "이미지",
            "사진묶기",
            "pdf합치기",
            "pdf페이지",
            "pdf회전",
            "깨짐",
            "메일",
            "메일첨부",
            "메일보내기",
        ] {
            assert!(p.contains(cmd), "시스템 프롬프트에 '{cmd}' 안내가 없습니다");
        }
    }

    #[test]
    fn notify_lists_all_push_channels_incl_kakao() {
        // notify 안내는 한국 1순위 채널 카카오를 포함한 4채널을 모두 알려야 한다
        // (configured_channels와 일치) — 안 그러면 "카카오로 보내줘"를 놓친다.
        let p = system_prompt(None, None);
        for ch in ["카카오", "디스코드", "슬랙", "텔레그램"] {
            assert!(p.contains(ch), "notify 안내에 '{ch}' 채널이 없습니다");
        }
    }
}

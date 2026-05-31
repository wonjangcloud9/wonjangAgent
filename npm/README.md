# wonjang-agent

> 원장 — 로컬 환경을 다루는 **한국어 우선** 자율 AI 에이전트 (Rust).

```bash
npm install -g wonjang-agent
```

설치하면 `wonjang` 명령을 바로 쓸 수 있습니다(설치 시 플랫폼에 맞는 네이티브
바이너리를 자동으로 내려받습니다).

```bash
export OPENROUTER_API_KEY=sk-or-...
wonjang                       # 대화형 모드
wonjang "git 상태 알려줘"        # 단발 실행
wonjang preset run 다운로드정리   # 한국어 작업 프리셋
```

자세한 사용법과 소스는 GitHub 저장소를 참고하세요:
**https://github.com/wonjangcloud9/wonjangAgent**

## 지원 플랫폼

| OS | 아키텍처 |
| --- | --- |
| macOS | arm64 (Apple Silicon), x64 |
| Linux | x64 (musl) |
| Windows | x64 |

지원되지 않는 플랫폼은 소스 빌드를 사용하세요:

```bash
cargo install --git https://github.com/wonjangcloud9/wonjangAgent
```

## 라이선스

MIT

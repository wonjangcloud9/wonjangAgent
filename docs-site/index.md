---
layout: home

hero:
  name: 원장
  text: 한국인을 위한 터미널 개인 비서
  tagline: 가계부·실수령·디데이·자랑 카드까지 — 키도 로그인도 없이 설치하면 바로. 단일 바이너리(Rust)라 빠릅니다.
  actions:
    - theme: brand
      text: 1분 설치
      link: /guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/wonjangcloud9/wonjangAgent
    - theme: alt
      text: npm
      link: https://www.npmjs.com/package/wonjang-agent

features:
  - icon: 💰
    title: 돈 — 매일 묻는 것
    details: 연봉 실수령액·가계부·대출·퇴직금·연차수당·사업자번호 검증까지. 4대 보험·소득세 떼고 월 실수령을 한 줄로.
    link: /features/money
    linkText: 돈 기능 보기
  - icon: 📅
    title: 시간·날짜
    details: 디데이·기념일·월급날·전역일·만 나이. 한국식 날짜(2026.11.19)도 그대로 알아들어요.
    link: /features/time
    linkText: 시간 기능 보기
  - icon: 🎉
    title: 자랑 카드
    details: 한 달의 나를 카톡에서도 안 깨지는 카드 한 장으로. --복사로 클립보드에 바로.
    link: /brag
    linkText: 자랑 카드 보기
  - icon: 🎭
    title: 내 원장, 내 성격으로
    details: 첫 실행에 성격을 고르면, 명령마다 원장이 자기 얼굴·말투로 응답해요.
    link: /character
    linkText: 캐릭터 보기
  - icon: 🇰🇷
    title: 한국 생활 유틸
    details: 평수·전통 단위·한글 금액·여권 로마자·한영 오타 복구까지. 한국 사람이 쓰는 걸 그대로.
    link: /features/life
    linkText: 생활 유틸 보기
  - icon: ⚡
    title: 키 없이 바로
    details: 핵심 기능은 API 키도 로그인도 없이. 단일 바이너리(Rust)·musl 정적이라 빠르고 가벼워요.
    link: /guide/getting-started
    linkText: 지금 설치
---

## 키 없이, 지금 바로

```bash
npm install -g wonjang-agent
```

```bash
wonjang                       # 대화형 — 먼저 성격을 골라요
wonjang 자랑                   # 한 달의 나를 카톡 카드로
wonjang 연봉 3600              # 월 실수령액
wonjang 지출 추가 5만 식비       # 가계부
```

금액은 **한국식 그대로** 알아들어요: `5만` · `1억` · `1.5억` · `50,000` 다 OK.

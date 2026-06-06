import { defineConfig } from 'vitepress'

// 원장(wonjang) 공식 문서 — 한국어 단일 로케일, VitePress 기본 테마.
export default defineConfig({
  lang: 'ko-KR',
  title: '원장 (wonjang)',
  description:
    '한국인을 위한 터미널 개인 비서. 가계부·실수령액·디데이·전역일·자랑 카드·날씨·환율까지 키도 로그인도 없이 설치하면 바로. 단일 바이너리(Rust).',
  base: '/wonjangAgent/',
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ['meta', { name: 'theme-color', content: '#0d1117' }],
    ['meta', { name: 'author', content: 'wonjang' }],
    [
      'meta',
      {
        name: 'keywords',
        content:
          '원장, wonjang, 터미널 비서, CLI 개인비서, 한국어 CLI, 실수령액 계산기, 가계부 CLI, 디데이 계산기, 전역일 계산기, 자랑 카드, 사업자번호 검증, Rust CLI, 개인 비서, AI 비서',
      },
    ],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:locale', content: 'ko_KR' }],
    ['meta', { property: 'og:site_name', content: '원장(wonjang)' }],
    ['meta', { property: 'og:title', content: '원장 — 한국인을 위한 터미널 개인 비서' }],
    [
      'meta',
      {
        property: 'og:description',
        content: '가계부·실수령·디데이·자랑 카드까지. 키 없이 설치하면 바로 쓰는 한국어 CLI 비서.',
      },
    ],
    ['meta', { property: 'og:url', content: 'https://wonjangcloud9.github.io/wonjangAgent/' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['link', { rel: 'canonical', href: 'https://wonjangcloud9.github.io/wonjangAgent/' }],
  ],
  sitemap: {
    hostname: 'https://wonjangcloud9.github.io/wonjangAgent/',
  },
  themeConfig: {
    nav: [
      { text: '시작하기', link: '/guide/getting-started' },
      {
        text: '기능',
        items: [
          { text: '💰 돈', link: '/features/money' },
          { text: '📅 시간·날짜', link: '/features/time' },
          { text: '🇰🇷 한국 생활', link: '/features/life' },
          { text: '🌐 실시간 정보', link: '/features/realtime' },
          { text: '🛠 도구', link: '/features/tools' },
        ],
      },
      { text: '자랑 카드', link: '/brag' },
      { text: '캐릭터', link: '/character' },
      { text: 'npm', link: 'https://www.npmjs.com/package/wonjang-agent' },
    ],
    sidebar: [
      {
        text: '시작',
        items: [
          { text: '시작하기', link: '/guide/getting-started' },
          { text: '설치', link: '/guide/install' },
        ],
      },
      {
        text: '기능',
        items: [
          { text: '💰 돈 — 매일 묻는 것', link: '/features/money' },
          { text: '📅 시간·날짜', link: '/features/time' },
          { text: '🇰🇷 한국 생활 유틸', link: '/features/life' },
          { text: '🌐 실시간 정보', link: '/features/realtime' },
          { text: '🛠 도구', link: '/features/tools' },
        ],
      },
      {
        text: '자랑·캐릭터',
        items: [
          { text: '🎉 자랑 카드', link: '/brag' },
          { text: '🎭 캐릭터(성격)', link: '/character' },
        ],
      },
      {
        text: '확장',
        items: [{ text: '🤖 AI로 확장', link: '/ai' }],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/wonjangcloud9/wonjangAgent' }],
    search: { provider: 'local' },
    footer: {
      message: 'MIT License로 배포됩니다.',
      copyright: '© 2026 wonjang — 한국인을 위한 터미널 개인 비서',
    },
    docFooter: { prev: '이전', next: '다음' },
    outline: { label: '이 페이지', level: [2, 3] },
    darkModeSwitchLabel: '다크 모드',
    lightModeSwitchTitle: '라이트 모드로',
    darkModeSwitchTitle: '다크 모드로',
    sidebarMenuLabel: '메뉴',
    returnToTopLabel: '맨 위로',
    langMenuLabel: '언어',
    lastUpdatedText: '마지막 업데이트',
  },
})

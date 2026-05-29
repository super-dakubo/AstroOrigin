export const THEMES = {
  genshin: {
    name: '原神',
    appName: '旅行者笔记',
    emoji: '⛰️',
    primary: '#FFD700',
    accent: '#60A5FA',
    gold: '#FFD700',
    bg: '#FFF7E6',
    text: '#1E3A8A',
    border: '#E8DDD0',
    barGradient: 'linear-gradient(90deg, #FFD700, #60A5FA)',
    isDark: false,
  },
  starrail: {
    name: '星铁',
    appName: '开拓者日志',
    emoji: '🚂',
    primary: '#A855F7',
    accent: '#FDE047',
    gold: '#FDE047',
    bg: '#F5F3FF',
    text: '#1E1B4B',
    border: '#E0DCF5',
    barGradient: 'linear-gradient(90deg, #A855F7, #FDE047)',
    isDark: false,
  },
} as const;

export type GameKind = keyof typeof THEMES;

export const ROUTES = {
  OVERVIEW: '/',
  GACHA: '/gacha',
  PLAYTIME: '/playtime',
  SCREENSHOTS: '/screenshots',
} as const;

export const THEMES = {
  genshin: {
    name: '原神',
    emoji: '⛰️',
    primary: '#D4433B',
    gold: '#C89B3C',
    bg: '#FAFAF7',
    border: '#F0E4D8',
    barGradient: 'linear-gradient(90deg, #D4433B, #C89B3C)',
  },
  starrail: {
    name: '星铁',
    emoji: '🚂',
    primary: '#3D5A80',
    gold: '#C89B3C',
    bg: '#F5F7FA',
    border: '#DCE0E8',
    barGradient: 'linear-gradient(90deg, #3D5A80, #C89B3C)',
  },
} as const;

export type GameKind = keyof typeof THEMES;

export const ROUTES = {
  OVERVIEW: '/',
  GACHA: '/gacha',
  PLAYTIME: '/playtime',
  SCREENSHOTS: '/screenshots',
} as const;

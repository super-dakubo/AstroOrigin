import { create } from 'zustand';
import type { GameKind } from '../lib/constants';
import { THEMES } from '../lib/constants';

interface GameState {
  currentGame: GameKind;
  setGame: (game: GameKind) => void;
  theme: typeof THEMES[GameKind];
}

export const useGameStore = create<GameState>((set) => ({
  currentGame: 'genshin',
  setGame: (game) => set({ currentGame: game, theme: THEMES[game] }),
  theme: THEMES.genshin,
}));

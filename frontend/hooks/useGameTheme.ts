import { useEffect } from 'react';
import { useGameStore } from '../stores/gameStore';

export function useGameTheme() {
  const theme = useGameStore((s) => s.theme);

  useEffect(() => {
    document.documentElement.style.setProperty('--theme-primary', theme.primary);
    document.documentElement.style.setProperty('--theme-gold', theme.gold);
    document.documentElement.style.setProperty('--theme-bg', theme.bg);
  }, [theme]);

  return theme;
}

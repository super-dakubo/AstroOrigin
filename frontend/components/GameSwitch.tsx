import { useGameStore } from '../stores/gameStore';
import type { GameKind } from '../lib/constants';

export function GameSwitch() {
  const { currentGame, setGame } = useGameStore();
  const isDark = currentGame === 'starrail';

  const games: { key: GameKind; label: string }[] = [
    { key: 'genshin', label: '⛰️ 原神' },
    { key: 'starrail', label: '🚂 星铁' },
  ];

  return (
    <div className={`inline-flex rounded-lg p-0.5 ${isDark ? 'bg-slate-800' : 'bg-gray-100'}`}>
      {games.map((g) => (
        <button
          key={g.key}
          onClick={() => setGame(g.key)}
          className={`px-4 py-1.5 text-sm rounded-md transition-all ${
            currentGame === g.key
              ? isDark
                ? 'bg-slate-700 shadow-sm font-medium text-white'
                : 'bg-white shadow-sm font-medium text-gray-900'
              : isDark
                ? 'text-slate-400 hover:text-slate-200'
                : 'text-gray-500 hover:text-gray-700'
          }`}
        >
          {g.label}
        </button>
      ))}
    </div>
  );
}

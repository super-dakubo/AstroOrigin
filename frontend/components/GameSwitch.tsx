import { useGameStore } from '../stores/gameStore'
import type { GameKind } from '../lib/constants'

const games: { key: GameKind; label: string }[] = [
  { key: 'genshin', label: '原神' },
  { key: 'starrail', label: '星铁' }
]

export function GameSwitch() {
  const { currentGame, setGame } = useGameStore()

  return (
    <div className="inline-flex bg-gray-100 rounded-lg p-0.5">
      {games.map((g) => (
        <button
          key={g.key}
          onClick={() => setGame(g.key)}
          className={`px-4 py-1.5 text-sm rounded-md transition-colors ${
            currentGame === g.key
              ? 'bg-white shadow-sm font-medium text-gray-900'
              : 'text-gray-500 hover:text-gray-700'
          }`}
        >
          {g.label}
        </button>
      ))}
    </div>
  )
}

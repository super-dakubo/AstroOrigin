import { useGameStore } from '../stores/gameStore';

export function Screenshots() {
  const currentGame = useGameStore((s) => s.currentGame);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">截图策展</h1>
        <p className="text-sm text-gray-500 mt-1">
          {currentGame === 'genshin'
            ? '旅行者，你的美好瞬间'
            : '开拓者，你的旅途记忆'}
        </p>
      </div>
      <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">
        截图智能策展将在后续版本中实现
      </div>
    </div>
  );
}

import { useGameStore } from '../stores/gameStore';

export function Screenshots() {
  const currentGame = useGameStore((s) => s.currentGame);

  const isDark = currentGame === 'starrail';
  const headingClass = isDark ? 'text-white' : 'text-gray-900';
  const subClass = isDark ? 'text-slate-400' : 'text-gray-500';

  return (
    <div className="space-y-6">
      <div>
        <h1 className={`text-2xl font-bold ${headingClass}`}>截图策展</h1>
        <p className={`text-sm ${subClass} mt-1`}>
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

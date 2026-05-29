import { useGameStore } from '../stores/gameStore';

export function Playtime() {
  const currentGame = useGameStore((s) => s.currentGame);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">游戏时长</h1>
        <p className="text-sm text-gray-500 mt-1">
          {currentGame === 'genshin'
            ? '旅行者，看看你今天玩了多久'
            : '开拓者，看看你今天开拓了多久'}
        </p>
      </div>
      <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">
        游戏时长统计将在后续版本中实现
      </div>
    </div>
  );
}

import { useGameStore } from '../stores/gameStore';

export function Gacha() {
  const currentGame = useGameStore((s) => s.currentGame);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">抽卡记录</h1>
          <p className="text-sm text-gray-500 mt-1">
            {currentGame === 'genshin' ? '派蒙帮你记着每一抽' : '帕姆帮你记着每一跃'}
          </p>
        </div>
      </div>
      <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">
        导入截图后将在此展示抽卡记录
      </div>
    </div>
  );
}

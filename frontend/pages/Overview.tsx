import { useGameStore } from '../stores/gameStore';

export function Overview() {
  const currentGame = useGameStore((s) => s.currentGame);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">总览</h1>
        <p className="text-sm text-gray-500 mt-1">
          {currentGame === 'genshin' ? '旅行者，来看看你的战绩' : '开拓者，来看看你的战绩'}
        </p>
      </div>
      <div className="grid grid-cols-4 gap-4">
        {['累计抽数', '5⭐ 出货', '当前保底', '本月在线'].map((label) => (
          <div key={label} className="bg-white rounded-xl border border-gray-200 p-4">
            <div className="text-xs text-gray-400">{label}</div>
            <div className="text-2xl font-bold text-gray-900 mt-1">--</div>
          </div>
        ))}
      </div>
    </div>
  );
}

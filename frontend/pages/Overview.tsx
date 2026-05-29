import { useTauriQuery } from '../hooks/useTauriQuery';
import { useGameStore } from '../stores/gameStore';
import { StatCard } from '../components/StatCard';

interface GachaStats {
  totalPulls: number;
  fiveStarCount: number;
  lostCount: number;
  currentPity: number;
  avgPullsPerFiveStar: number;
}

export function Overview() {
  const currentGame = useGameStore((s) => s.currentGame);
  const theme = useGameStore((s) => s.theme);

  const { data: genshinStats } = useTauriQuery<GachaStats>('get_gacha_stats', {
    gameKind: 'genshin',
  });
  const { data: starrailStats } = useTauriQuery<GachaStats>('get_gacha_stats', {
    gameKind: 'starrail',
  });

  const currentStats = currentGame === 'genshin' ? genshinStats : starrailStats;
  const lostRate = currentStats && currentStats.fiveStarCount > 0
    ? Math.round((currentStats.lostCount / currentStats.fiveStarCount) * 100)
    : 0;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">总览</h1>
        <p className="text-sm text-gray-500 mt-1">
          {currentGame === 'genshin'
            ? '旅行者，来看看你的战绩'
            : '开拓者，来看看你的战绩'}
        </p>
      </div>

      <div className="grid grid-cols-4 gap-4">
        <StatCard
          label="累计抽数"
          value={currentStats?.totalPulls?.toLocaleString() ?? '--'}
        />
        <StatCard
          label="5⭐ 出货"
          value={currentStats?.fiveStarCount ?? '--'}
          sub={currentStats ? `平均 ${currentStats.avgPullsPerFiveStar.toFixed(1)} 抽` : undefined}
          subColor={theme.primary}
        />
        <StatCard
          label="当前保底"
          value={currentStats?.currentPity ?? '--'}
          sub={currentStats ? `距保底 ${90 - currentStats.currentPity} 抽` : undefined}
          subColor="#D4433B"
        />
        <StatCard
          label="歪率"
          value={currentStats ? `${lostRate}%` : '--'}
          sub={currentStats ? `${currentStats.lostCount} / ${currentStats.fiveStarCount}` : undefined}
          subColor="#D4433B"
        />
      </div>

      {/* Quick game switcher cards */}
      <div className="grid grid-cols-2 gap-4">
        <div
          className="rounded-xl border p-4 cursor-pointer transition-all"
          style={{
            background: currentGame === 'genshin' ? '#fff' : '#f9f9f9',
            borderColor: currentGame === 'genshin' ? '#D4433B' : '#e5e7eb',
          }}
          onClick={() => useGameStore.getState().setGame('genshin')}
        >
          <div className="text-sm font-semibold text-gray-900">⛰️ 原神</div>
          <div className="text-xs text-gray-400 mt-1">
            {genshinStats
              ? `${genshinStats.totalPulls} 抽 · ${genshinStats.fiveStarCount} 个5⭐`
              : '加载中...'}
          </div>
        </div>
        <div
          className="rounded-xl border p-4 cursor-pointer transition-all"
          style={{
            background: currentGame === 'starrail' ? '#fff' : '#f9f9f9',
            borderColor: currentGame === 'starrail' ? '#3D5A80' : '#e5e7eb',
          }}
          onClick={() => useGameStore.getState().setGame('starrail')}
        >
          <div className="text-sm font-semibold text-gray-900">🚂 星铁</div>
          <div className="text-xs text-gray-400 mt-1">
            {starrailStats
              ? `${starrailStats.totalPulls} 抽 · ${starrailStats.fiveStarCount} 个5⭐`
              : '加载中...'}
          </div>
        </div>
      </div>
    </div>
  );
}

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

  const { data: stats } = useTauriQuery<GachaStats>('get_gacha_stats', {
    gameKind: currentGame,
  });

  const lostRate = stats && stats.fiveStarCount > 0
    ? Math.round((stats.lostCount / stats.fiveStarCount) * 100)
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
          value={stats?.totalPulls?.toLocaleString() ?? '--'}
        />
        <StatCard
          label="5⭐ 出货"
          value={stats?.fiveStarCount ?? '--'}
          sub={stats ? `平均 ${stats.avgPullsPerFiveStar.toFixed(1)} 抽` : undefined}
          subColor={theme.primary}
        />
        <StatCard
          label="当前保底"
          value={stats?.currentPity ?? '--'}
          sub={stats ? `距保底 ${90 - stats.currentPity} 抽` : undefined}
          subColor="#D4433B"
        />
        <StatCard
          label="歪率"
          value={stats ? `${lostRate}%` : '--'}
          sub={stats ? `${stats.lostCount} / ${stats.fiveStarCount}` : undefined}
          subColor="#D4433B"
        />
      </div>
    </div>
  );
}

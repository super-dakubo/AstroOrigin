import { useTauriQuery, useTauriMutation } from '../hooks/useTauriQuery';
import { useGameStore } from '../stores/gameStore';
import { StatCard } from '../components/StatCard';
import { LuckChart } from '../components/LuckChart';
import { RecordTable } from '../components/RecordTable';
import { open } from '@tauri-apps/plugin-dialog';

interface GachaStats {
  totalPulls: number;
  fiveStarCount: number;
  lostCount: number;
  currentPity: number;
  avgPullsPerFiveStar: number;
}

interface GachaRecord {
  id: number;
  gameKind: string;
  itemName: string;
  starRating: number;
  recordDate: string;
  isWon: boolean;
}

export function Gacha() {
  const currentGame = useGameStore((s) => s.currentGame);
  const theme = useGameStore((s) => s.theme);

  const { data: stats, refetch: refetchStats } = useTauriQuery<GachaStats>('get_gacha_stats', {
    gameKind: currentGame,
  });
  const { data: records, refetch: refetchRecords } = useTauriQuery<GachaRecord[]>('get_gacha_records', {
    gameKind: currentGame,
    limit: 200,
  });

  const importMutation = useTauriMutation<{ imported: number; duplicates: number }, { imagePath: string; gameKind: string }>('import_gacha_screenshot');

  const handleImport = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: '截图', extensions: ['png', 'jpg', 'jpeg', 'bmp'] }],
    });
    if (!selected) return;

    try {
      const result = await importMutation.mutateAsync({
        imagePath: selected,
        gameKind: currentGame,
      });
      refetchStats();
      refetchRecords();
      alert(`导入成功！新增 ${result.imported} 条，跳过 ${result.duplicates} 条重复`);
    } catch (e) {
      alert(`导入失败：${e}`);
    }
  };

  const chartData = (records ?? [])
    .filter((r) => r.starRating === 5)
    .map((r, i, arr) => ({
      pulls: i === 0 ? records!.filter((rec) => rec.id < r.id).length + 1 : Math.abs(
        records!.filter((rec) => rec.id <= r.id && rec.starRating === 5).length -
        records!.filter((rec) => rec.id <= (arr[i-1]?.id ?? 0) && rec.starRating === 5).length
      ) || 1,
      isFiveStar: true,
      isWon: r.isWon,
    }));

  const lostRate = stats && stats.fiveStarCount > 0
    ? Math.round((stats.lostCount / stats.fiveStarCount) * 100)
    : 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">抽卡记录</h1>
          <p className="text-sm text-gray-500 mt-1">
            {currentGame === 'genshin' ? '派蒙帮你记着每一抽' : '帕姆帮你记着每一跃'}
          </p>
        </div>
        <button
          onClick={handleImport}
          disabled={importMutation.isPending}
          className="px-4 py-2 text-sm text-white font-medium rounded-lg transition-colors disabled:opacity-50"
          style={{ background: theme.primary }}
        >
          {importMutation.isPending ? '导入中...' : '+ 导入截图'}
        </button>
      </div>

      <div className="grid grid-cols-4 gap-4">
        <StatCard label="累计抽数" value={stats?.totalPulls?.toLocaleString() ?? '--'} />
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
          sub={stats ? `${stats.lostCount} / ${stats.fiveStarCount} 歪了` : undefined}
          subColor="#D4433B"
        />
      </div>

      <LuckChart records={chartData} />
      <RecordTable records={records ?? []} />
    </div>
  );
}

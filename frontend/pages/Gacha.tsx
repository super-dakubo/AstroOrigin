import { useTauriQuery, useTauriMutation } from '../hooks/useTauriQuery';
import { useGameStore } from '../stores/gameStore';
import { StatCard } from '../components/StatCard';
import { LuckChart } from '../components/LuckChart';
import { RecordTable } from '../components/RecordTable';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { useState, useEffect, useRef } from 'react';

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

interface ImportProgress {
  current: number;
  total: number;
  file?: string;
  status?: string;
  done?: boolean;
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
  const batchImportMutation = useTauriMutation<{ imported: number; duplicates: number }, { imagePaths: string[]; gameKind: string }>('import_gacha_screenshots');
  const deleteMutation = useTauriMutation<boolean, { id: number }>('delete_gacha_record');
  const updateMutation = useTauriMutation<boolean, { id: number; itemName: string; starRating: number; recordDate: string; isWon: boolean }>('update_gacha_record');
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [progressExpanded, setProgressExpanded] = useState(false);
  const [currentFile, setCurrentFile] = useState<string>('');
  const unlistenRef = useRef<(() => void) | null>(null);

  // 监听导入进度事件
  useEffect(() => {
    const setup = async () => {
      if (unlistenRef.current) unlistenRef.current();
      unlistenRef.current = await listen<ImportProgress>('import-progress', (event) => {
        const p = event.payload;
        if (p.status === 'processing') {
          setCurrentFile(p.file || '');
        }
        setProgress(p);
        if (p.done) {
          setTimeout(() => { setProgressExpanded(false); setCurrentFile(''); }, 2000);
        }
      });
    };
    setup();
    return () => { if (unlistenRef.current) unlistenRef.current(); };
  }, []);

  const handleSingleImport = async () => {
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
      setError(null);
      alert(`导入成功！新增 ${result.imported} 条，跳过 ${result.duplicates} 条重复`);
    } catch (e) {
      const msg = `导入失败：${e}`;
      setError(msg);
      console.error(msg);
    }
  };

  const handleBatchImport = async () => {
    const selected = await open({
      multiple: true,
      filters: [{ name: '截图', extensions: ['png', 'jpg', 'jpeg', 'bmp'] }],
    });
    if (!selected || selected.length === 0) return;

    setProgress({ current: 0, total: selected.length });

    try {
      const result = await batchImportMutation.mutateAsync({
        imagePaths: selected,
        gameKind: currentGame,
      });
      refetchStats();
      refetchRecords();
      setError(null);
      setProgress(null);
      alert(`批量导入完成！新增 ${result.imported} 条，跳过 ${result.duplicates} 条`);
    } catch (e) {
      const msg = `批量导入失败：${e}`;
      setError(msg);
      console.error(msg);
      setProgress(null);
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

  const isImporting = importMutation.isPending || batchImportMutation.isPending;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">抽卡记录</h1>
          <p className="text-sm text-gray-500 mt-1">
            {currentGame === 'genshin' ? '派蒙帮你记着每一抽' : '帕姆帮你记着每一跃'}
          </p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleSingleImport}
            disabled={isImporting}
            className="px-4 py-2 text-sm text-white font-medium rounded-lg transition-colors disabled:opacity-50"
            style={{ background: theme.primary }}
          >
            {isImporting ? '导入中...' : '+ 导入截图'}
          </button>
          <button
            onClick={handleBatchImport}
            disabled={isImporting}
            className="px-4 py-2 text-sm text-white font-medium rounded-lg transition-colors disabled:opacity-50 border-2"
            style={{ background: theme.primary, borderColor: theme.primary }}
          >
            {isImporting ? '导入中...' : '+ 批量导入'}
          </button>
        </div>
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

      {/* 进度：默认紧凑百分比，点击展开详情 */}
      {progress && !progress.done && (
        <div className="flex items-center gap-3">
          <div
            className="flex items-center gap-2 cursor-pointer select-none"
            onClick={() => setProgressExpanded(!progressExpanded)}
          >
            <div className="relative w-8 h-8">
              <svg className="w-8 h-8 -rotate-90" viewBox="0 0 32 32">
                <circle cx="16" cy="16" r="14" fill="none" stroke="#e5e7eb" strokeWidth="3" />
                <circle
                  cx="16" cy="16" r="14" fill="none"
                  stroke="#3b82f6" strokeWidth="3" strokeLinecap="round"
                  strokeDasharray={`${(progress.current / progress.total) * 87.96} 87.96`}
                />
              </svg>
              <span className="absolute inset-0 flex items-center justify-center text-[10px] font-semibold text-blue-600">
                {Math.round((progress.current / progress.total) * 100)}%
              </span>
            </div>
            <span className="text-sm text-gray-600">
              {progressExpanded ? '收起' : '导入中...'}
            </span>
          </div>

          {/* 展开详情 */}
          {progressExpanded && (
            <div className="flex-1 bg-blue-50 border border-blue-200 rounded-lg p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm text-blue-700">
                  {currentFile ? currentFile.split(/[/\\]/).pop() : ''} ({progress.current}/{progress.total})
                </span>
              </div>
              <div className="w-full bg-blue-200 rounded-full h-2">
                <div
                  className="bg-blue-500 h-2 rounded-full transition-all duration-200"
                  style={{ width: `${(progress.current / progress.total) * 100}%` }}
                />
              </div>
            </div>
          )}
        </div>
      )}

      <LuckChart records={chartData} />
      <RecordTable
        records={records ?? []}
        onDelete={async (id) => {
          await deleteMutation.mutateAsync({ id });
          refetchStats();
          refetchRecords();
        }}
        onSave={async (id, data) => {
          await updateMutation.mutateAsync({ id, ...data });
          refetchStats();
          refetchRecords();
        }}
      />

      {error && (
        <div className="fixed bottom-4 right-4 z-50 max-w-md">
          <div className="bg-red-50 border border-red-200 rounded-lg p-3 shadow-lg">
            <div className="flex items-center justify-between mb-1">
              <span className="text-xs font-medium text-red-600">错误</span>
              <button onClick={() => setError(null)} className="text-red-400 hover:text-red-600 text-xs">✕</button>
            </div>
            <textarea
              readOnly
              value={error}
              className="w-full text-xs text-red-700 bg-transparent border-none resize-none outline-none"
              rows={3}
              onClick={(e) => (e.target as HTMLTextAreaElement).select()}
            />
          </div>
        </div>
      )}
    </div>
  );
}

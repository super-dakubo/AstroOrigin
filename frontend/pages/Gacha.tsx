import { useTauriQuery, useTauriMutation } from '../hooks/useTauriQuery'
import { useGameStore } from '../stores/gameStore'
import { StatCard } from '../components/StatCard'
import { LuckChart } from '../components/LuckChart'
import { RecordTable } from '../components/RecordTable'
import { open } from '@tauri-apps/plugin-dialog'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useState, useEffect, useRef } from 'react'

interface BannerStats {
  bannerType: string
  totalPulls: number
  fiveStarCount: number
  lostCount: number
  currentPity: number
  avgPullsPerFiveStar: number
}

interface GachaStats {
  totalPulls: number
  fiveStarCount: number
  lostCount: number
  currentPity: number
  avgPullsPerFiveStar: number
  byBanner: BannerStats[]
}

interface GachaRecord {
  id: number
  gameKind: string
  itemName: string
  itemType: string
  bannerType: string
  starRating: number
  recordDate: string
  isWon: boolean
}

interface GachaRecordsResponse {
  records: GachaRecord[]
  total: number
}

interface ImportProgress {
  current: number
  total: number
  file?: string
  phase?: string
  status?: string
  done?: boolean
}

const PHASE_LABELS: Record<string, string> = {
  detect: '🔍 文本检测',
  recognize: '📝 文字识别',
  parse: '📊 表格解析',
  save: '💾 入库中'
}

const BANNER_TABS: Record<string, string[]> = {
  starrail: ['全部', '角色活动', '光锥活动', '常驻'],
  genshin: ['全部', '角色活动', '武器活动', '常驻']
}

export function Gacha() {
  const currentGame = useGameStore((s) => s.currentGame)
  const theme = useGameStore((s) => s.theme)

  const [page, setPage] = useState(1)
  const [bannerTab, setBannerTab] = useState<string>('全部')
  const [starFilter, setStarFilter] = useState<number | null>(null)
  const [sortBy, setSortBy] = useState<string>('date')
  const [sortOrder, setSortOrder] = useState<string>('desc')
  const [pageSize, setPageSize] = useState<number>(20)

  const { data: stats, refetch: refetchStats } = useTauriQuery<GachaStats>('get_gacha_stats', {
    gameKind: currentGame
  })
  const { data: recordsResponse, refetch: refetchRecords } = useTauriQuery<GachaRecordsResponse>(
    'get_gacha_records',
    {
      gameKind: currentGame,
      page,
      pageSize,
      banner: bannerTab !== '全部' ? bannerTab : null,
      starFilter,
      sortBy,
      sortOrder
    }
  )

  // 切游戏时重置所有筛选状态
  useEffect(() => {
    setPage(1)
    setBannerTab('全部')
    setStarFilter(null)
    setSortBy('date')
    setSortOrder('desc')
    setPageSize(20)
  }, [currentGame])

  const importMutation = useTauriMutation<
    { imported: number; duplicates: number },
    { imagePath: string; gameKind: string }
  >('import_gacha_screenshot')
  const batchImportMutation = useTauriMutation<
    { imported: number; duplicates: number },
    { imagePaths: string[]; gameKind: string }
  >('import_gacha_screenshots')
  const deleteMutation = useTauriMutation<boolean, { id: number }>('delete_gacha_record')
  const updateMutation = useTauriMutation<
    boolean,
    {
      id: number
      itemName: string
      itemType: string
      bannerType: string
      starRating: number
      recordDate: string
      isWon: boolean
    }
  >('update_gacha_record')

  const [error, setError] = useState<string | null>(null)
  const [progress, setProgress] = useState<ImportProgress | null>(null)
  const [progressExpanded, setProgressExpanded] = useState(false)
  const [currentFile, setCurrentFile] = useState<string>('')
  const unlistenRef = useRef<UnlistenFn | null>(null)

  // 监听导入进度事件
  useEffect(() => {
    const setup = async () => {
      if (unlistenRef.current) unlistenRef.current()
      unlistenRef.current = await listen<ImportProgress>('import-progress', (event) => {
        const p = event.payload
        if (p.file) setCurrentFile(p.file)
        setProgress(p)
        if (p.done) {
          setTimeout(() => {
            setProgressExpanded(false)
            setCurrentFile('')
          }, 2000)
        }
      })
    }
    setup()
    return () => {
      if (unlistenRef.current) unlistenRef.current()
    }
  }, [])

  const handleSingleImport = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: '截图', extensions: ['png', 'jpg', 'jpeg', 'bmp'] }]
    })
    if (!selected) return

    try {
      const result = await importMutation.mutateAsync({
        imagePath: selected,
        gameKind: currentGame
      })
      refetchStats()
      refetchRecords()
      setError(null)
      alert(`导入成功！新增 ${result.imported} 条`)
    } catch (e) {
      const msg = `导入失败：${e}`
      setError(msg)
      console.error(msg)
    }
  }

  const handleBatchImport = async () => {
    const selected = await open({
      multiple: true,
      filters: [{ name: '截图', extensions: ['png', 'jpg', 'jpeg', 'bmp'] }]
    })
    if (!selected || selected.length === 0) return

    setProgress({ current: 0, total: selected.length })

    try {
      const result = await batchImportMutation.mutateAsync({
        imagePaths: selected,
        gameKind: currentGame
      })
      refetchStats()
      refetchRecords()
      setError(null)
      setProgress(null)
      alert(`批量导入完成！新增 ${result.imported} 条`)
    } catch (e) {
      const msg = `批量导入失败：${e}`
      setError(msg)
      console.error(msg)
      setProgress(null)
    }
  }

  const handleToggleWon = async (id: number, newIsWon: boolean) => {
    const record = records.find((r) => r.id === id)
    if (!record) return
    await updateMutation.mutateAsync({
      id,
      itemName: record.itemName,
      itemType: record.itemType,
      bannerType: record.bannerType,
      starRating: record.starRating,
      recordDate: record.recordDate,
      isWon: newIsWon
    })
    refetchStats()
    refetchRecords()
  }

  const records = recordsResponse?.records ?? []
  const total = recordsResponse?.total ?? 0

  const currentBannerStats =
    bannerTab !== '全部' ? stats?.byBanner?.find((b) => b.bannerType.includes(bannerTab)) : null

  const displayStats = currentBannerStats || stats

  const bannerBreakdown =
    stats?.byBanner
      ?.map((b) => `${b.bannerType.replace('跃迁', '').replace('祈愿', '')} ${b.totalPulls}`)
      .join(' / ') ?? ''

  const chartData = records
    .filter((r) => r.starRating === 5)
    .map((r, i, arr) => ({
      pulls:
        i === 0
          ? records.filter((rec) => rec.id < r.id).length + 1
          : Math.abs(
              records.filter((rec) => rec.id <= r.id && rec.starRating === 5).length -
                records.filter((rec) => rec.id <= (arr[i - 1]?.id ?? 0) && rec.starRating === 5)
                  .length
            ) || 1,
      isFiveStar: true,
      isWon: r.isWon
    }))

  const isImporting = importMutation.isPending || batchImportMutation.isPending

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

      {/* 卡池 Tabs */}
      <div className="flex gap-1 border-b border-gray-200 pb-2">
        {BANNER_TABS[currentGame === 'genshin' ? 'genshin' : 'starrail'].map((tab) => (
          <button
            key={tab}
            onClick={() => {
              setBannerTab(tab)
              setPage(1)
            }}
            className={`px-4 py-1.5 text-sm rounded-t-lg transition-colors ${
              bannerTab === tab
                ? 'bg-white text-gray-900 font-medium border border-b-0 border-gray-200 -mb-[1px]'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-4 gap-4">
        <StatCard
          label="累计抽数"
          value={displayStats?.totalPulls?.toLocaleString() ?? '--'}
          sub={bannerTab === '全部' && bannerBreakdown ? bannerBreakdown : undefined}
        />
        <StatCard
          label="5⭐ 出货"
          value={displayStats?.fiveStarCount ?? '--'}
          sub={
            displayStats
              ? displayStats.fiveStarCount > 0
                ? `平均 ${displayStats.avgPullsPerFiveStar.toFixed(1)} 抽`
                : '暂无出货'
              : undefined
          }
          subColor={theme.primary}
        />
        <StatCard
          label="当前保底"
          value={displayStats?.currentPity ?? '--'}
          sub={
            displayStats
              ? bannerTab.includes('光锥') || bannerTab.includes('武器')
                ? `距保底 ${80 - displayStats.currentPity} 抽`
                : `距保底 ${90 - displayStats.currentPity} 抽`
              : undefined
          }
          subColor="#D4433B"
        />
        <StatCard
          label="歪率"
          value={
            stats && stats.fiveStarCount > 0
              ? bannerTab !== '全部'
                ? currentBannerStats && currentBannerStats.fiveStarCount > 0
                  ? `${Math.round((currentBannerStats.lostCount / currentBannerStats.fiveStarCount) * 100)}%`
                  : '--'
                : `${Math.round((stats.lostCount / stats.fiveStarCount) * 100)}%`
              : '--'
          }
          sub={
            bannerTab !== '全部'
              ? currentBannerStats
                ? `${currentBannerStats.lostCount} / ${currentBannerStats.fiveStarCount}`
                : undefined
              : stats
                ? `${stats.lostCount} / ${stats.fiveStarCount} 歪了`
                : undefined
          }
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
                  cx="16"
                  cy="16"
                  r="14"
                  fill="none"
                  stroke="#3b82f6"
                  strokeWidth="3"
                  strokeLinecap="round"
                  strokeDasharray={`${(progress.current / progress.total) * 87.96} 87.96`}
                />
              </svg>
              <span className="absolute inset-0 flex items-center justify-center text-[10px] font-semibold text-blue-600">
                {Math.round((progress.current / progress.total) * 100)}%
              </span>
            </div>
            <span className="text-sm text-gray-600">{progressExpanded ? '收起' : '导入中...'}</span>
          </div>

          {/* 展开详情 */}
          {progressExpanded && (
            <div className="flex-1 bg-blue-50 border border-blue-200 rounded-lg p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm text-blue-700">
                  {currentFile ? currentFile.split(/[/\\]/).pop() : ''} ({progress.current}/
                  {progress.total})
                </span>
                <span className="text-xs text-blue-500">
                  {progress.phase ? PHASE_LABELS[progress.phase] || progress.phase : ''}
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

      {/* 筛选栏 */}
      <div className="flex gap-4 items-center flex-wrap">
        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-500">星级</label>
          <select
            value={starFilter ?? ''}
            onChange={(e) => {
              setStarFilter(e.target.value ? Number(e.target.value) : null)
              setPage(1)
            }}
            className="px-2 py-1 border border-gray-200 rounded-lg text-sm bg-white"
          >
            <option value="">全部</option>
            <option value="5">5★</option>
            <option value="4">4★</option>
            <option value="3">3★</option>
          </select>
        </div>

        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-500">排序</label>
          <select
            value={`${sortBy}-${sortOrder}`}
            onChange={(e) => {
              const [by, order] = e.target.value.split('-')
              setSortBy(by)
              setSortOrder(order)
              setPage(1)
            }}
            className="px-2 py-1 border border-gray-200 rounded-lg text-sm bg-white"
          >
            <option value="date-desc">日期 ↓</option>
            <option value="date-asc">日期 ↑</option>
            <option value="star-desc">星级 ↓</option>
            <option value="star-asc">星级 ↑</option>
          </select>
        </div>

        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-500">每页</label>
          <select
            value={pageSize}
            onChange={(e) => {
              setPageSize(Number(e.target.value))
              setPage(1)
            }}
            className="px-2 py-1 border border-gray-200 rounded-lg text-sm bg-white"
          >
            <option value={20}>20</option>
            <option value={50}>50</option>
            <option value={100}>100</option>
          </select>
        </div>
      </div>

      <LuckChart records={chartData} />
      <RecordTable
        records={records}
        total={total}
        page={page}
        pageSize={pageSize}
        onPageChange={setPage}
        onDelete={async (id) => {
          await deleteMutation.mutateAsync({ id })
          refetchStats()
          refetchRecords()
        }}
        onSave={async (id, data) => {
          await updateMutation.mutateAsync({ id, ...data })
          refetchStats()
          refetchRecords()
        }}
        onToggleWon={handleToggleWon}
      />

      {error && (
        <div className="fixed bottom-4 right-4 z-50 max-w-md">
          <div className="bg-red-50 border border-red-200 rounded-lg p-3 shadow-lg">
            <div className="flex items-center justify-between mb-1">
              <span className="text-xs font-medium text-red-600">错误</span>
              <button
                onClick={() => setError(null)}
                className="text-red-400 hover:text-red-600 text-xs"
              >
                ✕
              </button>
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
  )
}

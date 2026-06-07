import { useMemo, useState } from 'react'
import { useTauriQuery } from '../hooks/useTauriQuery'
import { useGameStore } from '../stores/gameStore'
import { StatCard } from '../components/StatCard'
import { FiveStarReview } from '../components/FiveStarReview'
import type { GachaRecord } from '../lib/types'

interface GachaStats {
  totalPulls: number
  fiveStarCount: number
  lostCount: number
  currentPity: number
  avgPullsPerFiveStar: number
}

const BANNER_TABS: Record<string, string[]> = {
  starrail: ['角色活动', '光锥活动', '常驻', '新手'],
  genshin: ['角色活动', '武器活动', '常驻', '集录']
}

export function Overview() {
  const currentGame = useGameStore((s) => s.currentGame)
  const theme = useGameStore((s) => s.theme)
  const [bannerTab, setBannerTab] = useState('角色活动')

  const { data: stats, isLoading } = useTauriQuery<GachaStats>('get_gacha_stats', {
    gameKind: currentGame
  })

  const { data: allRecords } = useTauriQuery<GachaRecord[]>('get_gacha_chart_records', {
    gameKind: currentGame
  })

  const lostRate =
    stats && stats.fiveStarCount > 0 ? Math.round((stats.lostCount / stats.fiveStarCount) * 100) : 0

  const displayRecords = useMemo(
    () => (allRecords ?? []).filter((r) => r.bannerType.includes(bannerTab)),
    [allRecords, bannerTab]
  )

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">总览</h1>
          <p className="text-sm text-gray-500 mt-1">
            {currentGame === 'genshin' ? '旅行者，来看看你的战绩' : '开拓者，来看看你的战绩'}
          </p>
        </div>
        <div className="grid grid-cols-4 gap-4">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="bg-white rounded-xl border border-gray-200 p-4 animate-pulse">
              <div className="h-3 bg-gray-200 rounded w-16 mb-3" />
              <div className="h-6 bg-gray-200 rounded w-20 mb-2" />
            </div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">总览</h1>
        <p className="text-sm text-gray-500 mt-1">
          {currentGame === 'genshin' ? '旅行者，来看看你的战绩' : '开拓者，来看看你的战绩'}
        </p>
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
          sub={stats ? `${stats.lostCount} / ${stats.fiveStarCount}` : undefined}
          subColor="#D4433B"
        />
      </div>

      <div className="flex gap-1 border-b border-gray-200 pb-2">
        {BANNER_TABS[currentGame].map((tab) => (
          <button
            key={tab}
            onClick={() => setBannerTab(tab)}
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

      <FiveStarReview records={displayRecords} />
    </div>
  )
}

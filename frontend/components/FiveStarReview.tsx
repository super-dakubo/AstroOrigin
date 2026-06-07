import { useMemo } from 'react'
import type { GachaRecord } from '../lib/types'

interface FiveStarReviewItem {
  id: number
  itemName: string
  starRating: number
  pulls: number // 显示用抽数（大保底 = 歪的 + 保底累积）
  rawPulls: number // 实际间隔抽数
  bannerType: string
  recordDate: string
  isWon: boolean | null
  isGuaranteed: boolean
  rating: string
}

interface FiveStarReviewProps {
  records: GachaRecord[]
}

const SR_STANDARD = new Set(['白露', '布洛妮娅', '姬子', '杰帕德', '克拉拉', '瓦尔特', '彦卿'])

const GI_STANDARD = new Set([
  '迪卢克',
  '琴',
  '刻晴',
  '莫娜',
  '七七',
  '提纳里',
  '迪希雅',
  '天空之刃',
  '天空之卷',
  '天空之翼',
  '天空之傲',
  '天空之脊',
  '风鹰剑',
  '和璞鸢',
  '阿莫斯之弓',
  '四风原典',
  '狼的末路',
  '贯虹之槊',
  '斫峰之刃',
  '无工之剑',
  '尘世之锁',
  '不灭月华'
])

function isLimitedBanner(bannerType: string): boolean {
  return (
    bannerType.includes('角色活动') ||
    bannerType.includes('武器活动') ||
    bannerType.includes('光锥活动')
  )
}

function isLost(gameKind: string, itemName: string, bannerType: string): boolean | null {
  if (!isLimitedBanner(bannerType)) return null
  const pool = gameKind === 'starrail' ? SR_STANDARD : GI_STANDARD
  return pool.has(itemName) || false
}

function getRating(pulls: number): string {
  if (pulls <= 10) return '⚡ 欧皇'
  if (pulls <= 30) return '✨ 欧'
  if (pulls <= 55) return '✅ 不错'
  if (pulls <= 75) return '正常'
  if (pulls <= 85) return '💀 非'
  return '💀 究极非酋'
}

function getPullStyle(pulls: number): { bg: string; text: string } {
  if (pulls <= 10) return { bg: 'bg-amber-50', text: 'text-amber-600' }
  if (pulls <= 30) return { bg: 'bg-emerald-50', text: 'text-emerald-600' }
  if (pulls <= 55) return { bg: 'bg-blue-50', text: 'text-blue-500' }
  if (pulls <= 75) return { bg: 'bg-gray-50', text: 'text-gray-500' }
  if (pulls <= 85) return { bg: 'bg-red-50', text: 'text-red-500' }
  return { bg: 'bg-red-100', text: 'text-red-700' }
}

export function FiveStarReview({ records }: FiveStarReviewProps) {
  const reviewItems = useMemo((): FiveStarReviewItem[] => {
    const fiveStars = records.filter((r) => r.starRating === 5).sort((a, b) => a.id - b.id)

    let lastLimitedLost = false
    let lastLostPulls = 0
    const items: FiveStarReviewItem[] = []
    const gameKind = records[0]?.gameKind || ''

    for (let i = 0; i < fiveStars.length; i++) {
      const r = fiveStars[i]
      const bannerType = r.bannerType || ''

      const prevId = i > 0 ? fiveStars[i - 1].id : 0
      const rawPulls = records.filter((rec) => rec.id > prevId && rec.id <= r.id).length

      // 先用启发式判定，若数据库 isWon=false（用户手动改过）则覆盖为歪
      let lost = isLimitedBanner(bannerType) ? isLost(gameKind, r.itemName, bannerType) : null
      if (lost !== null && !r.isWon) lost = true

      if (lost === true) {
        // 限定池歪了 — 记录歪的抽数
        items.push({
          id: r.id,
          itemName: r.itemName,
          starRating: 5,
          pulls: rawPulls,
          rawPulls,
          bannerType,
          recordDate: r.recordDate,
          isWon: false,
          isGuaranteed: false,
          rating: getRating(rawPulls)
        })
        lastLimitedLost = true
        lastLostPulls = rawPulls
      } else if (lastLimitedLost) {
        // 大保底出货 — 显示累积抽数（歪 + 保底）
        const totalPulls = lastLostPulls + rawPulls
        items.push({
          id: r.id,
          itemName: r.itemName,
          starRating: 5,
          pulls: totalPulls,
          rawPulls,
          bannerType,
          recordDate: r.recordDate,
          isWon: true,
          isGuaranteed: true,
          rating: getRating(totalPulls)
        })
        lastLimitedLost = false
        lastLostPulls = 0
      } else {
        // 没歪（限定小保底没歪 / 常驻/新手无 50/50）
        items.push({
          id: r.id,
          itemName: r.itemName,
          starRating: 5,
          pulls: rawPulls,
          rawPulls,
          bannerType,
          recordDate: r.recordDate,
          isWon: lost === null ? null : true,
          isGuaranteed: false,
          rating: getRating(rawPulls)
        })
        if (lost === false) lastLimitedLost = false
      }
    }

    return items.reverse()
  }, [records])

  const stats = useMemo(() => {
    const limitedItems = reviewItems.filter((i) => isLimitedBanner(i.bannerType))
    const totalPulls = limitedItems.reduce((s, i) => s + i.pulls, 0)
    const limitedTotal = limitedItems.length
    const wonCount = limitedItems.filter((i) => i.isWon === true && !i.isGuaranteed).length
    const totalNormal = limitedItems.filter((i) => !i.isGuaranteed).length
    const winRate = totalNormal > 0 ? Math.round((wonCount / totalNormal) * 100) : 0
    const longestPulls = limitedItems.reduce((max, i) => Math.max(max, i.pulls), 0)

    return {
      avg: limitedTotal > 0 ? (totalPulls / limitedTotal).toFixed(1) : '--',
      winRate,
      longest: longestPulls || '--'
    }
  }, [reviewItems])

  return (
    <div className="bg-white rounded-xl border border-gray-200 p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="text-sm font-semibold text-gray-900">5★ 出卡回顾</div>
        <div className="flex gap-4 text-xs text-gray-500">
          <span>平均 {stats.avg} 抽/5★</span>
          <span>小保底率 {stats.winRate}%</span>
          <span>最长 {stats.longest} 抽</span>
        </div>
      </div>

      {reviewItems.length === 0 ? (
        <div className="py-8 text-center text-gray-400 text-sm">暂无 5★ 出货记录</div>
      ) : (
        <div className="space-y-2">
          {reviewItems.map((item) => {
            const pullStyle = getPullStyle(item.pulls)

            // 歪/不歪文字（仅限定池有）
            const lostText =
              item.isWon === null
                ? null
                : item.isGuaranteed
                  ? `大保底（${item.pulls - item.rawPulls} + ${item.rawPulls}）`
                  : item.isWon
                    ? '没歪'
                    : '歪了'

            return (
              <div
                key={item.id}
                className={`group border border-gray-200 rounded-xl p-3 transition-shadow hover:shadow-md select-none`}
              >
                <div className="flex items-center gap-3 text-xs">
                  <span className="text-sm font-semibold text-gray-900 min-w-[72px] shrink-0">
                    {item.itemName}
                    <span className="text-amber-500 ml-1">★★★★★</span>
                  </span>
                  <span className="text-lg font-bold text-gray-900 min-w-[44px] text-center shrink-0">
                    {item.pulls}
                    <span className="text-xs font-normal text-gray-400 ml-0.5">抽</span>
                  </span>
                  {/* 欧非评级标签 — 基于抽数，所有卡池都有 */}
                  <span
                    className={`text-xs font-medium px-2 py-0.5 rounded shrink-0 ${pullStyle.bg} ${pullStyle.text}`}
                  >
                    {item.rating}
                  </span>

                  {/* hover 额外信息 */}
                  <span className="hidden group-hover:inline text-gray-300">|</span>
                  <span className="hidden group-hover:inline text-gray-500 truncate max-w-[140px]">
                    {item.bannerType}
                  </span>
                  {lostText && (
                    <>
                      <span className="hidden group-hover:inline text-gray-300">|</span>
                      <span
                        className={`hidden group-hover:inline shrink-0 ${
                          lostText === '歪了'
                            ? 'text-red-500'
                            : lostText === '大保底'
                              ? 'text-amber-500'
                              : 'text-gray-500'
                        }`}
                      >
                        {lostText}
                      </span>
                    </>
                  )}

                  <span className="text-xs text-gray-400 ml-auto shrink-0">{item.recordDate}</span>
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

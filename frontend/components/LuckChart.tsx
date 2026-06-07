import { useMemo } from 'react'
import { useECharts } from '../hooks/useECharts'
import { useGameStore } from '../stores/gameStore'
import type { EChartsOption } from '../lib/echarts'

interface LuckChartProps {
  records: Array<{ pulls: number; isFiveStar: boolean; isWon?: boolean; has5050?: boolean }>
}

export function LuckChart({ records }: LuckChartProps) {
  const theme = useGameStore((s) => s.theme)

  // 在所有条件 return 之前调用所有 hooks
  const option: EChartsOption = useMemo(
    () => ({
      tooltip: { trigger: 'item' },
      grid: { left: 40, right: 16, top: 16, bottom: 24 },
      xAxis: {
        type: 'category',
        data: records.map((_, i) => i + 1),
        axisLabel: { fontSize: 10, color: '#9ca3af' }
      },
      yAxis: {
        type: 'value',
        name: '抽数间隔',
        nameTextStyle: { fontSize: 10, color: '#9ca3af' },
        axisLabel: { fontSize: 10, color: '#9ca3af' }
      },
      series: [
        {
          type: 'bar',
          data: records.map((r) => ({
            value: r.pulls,
            itemStyle: {
              color: r.isFiveStar
                ? r.has5050
                  ? r.isWon === false
                    ? '#D4433B'
                    : theme.gold
                  : '#94a3b8'
                : '#e5e7eb'
            }
          })),
          barMaxWidth: 20
        }
      ]
    }),
    [records, theme.gold]
  )

  const chartRef = useECharts(option)

  if (records.length === 0) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 p-4">
        <div className="text-sm font-semibold text-gray-900 mb-3">欧非曲线</div>
        <div className="h-48 flex items-center justify-center text-gray-400 text-sm">暂无数据</div>
      </div>
    )
  }

  return (
    <div className="bg-white rounded-xl border border-gray-200 p-4">
      <div className="flex items-center justify-between mb-3">
        <div>
          <div className="text-sm font-semibold text-gray-900">欧非曲线</div>
          <div className="text-xs text-gray-400">
            {records[0]?.has5050 ? '金色 = 5⭐ · 红色 = 歪了' : '灰色 = 5⭐（无 50/50）'}
          </div>
        </div>
      </div>
      <div ref={chartRef} className="w-full h-48" />
    </div>
  )
}

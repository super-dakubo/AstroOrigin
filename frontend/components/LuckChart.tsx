import { useECharts } from '../hooks/useECharts'
import { useGameStore } from '../stores/gameStore'
import type { EChartsOption } from '../lib/echarts'

interface LuckChartProps {
  records: Array<{ pulls: number; isFiveStar: boolean; isWon?: boolean }>
}

export function LuckChart({ records }: LuckChartProps) {
  const theme = useGameStore((s) => s.theme)

  const option: EChartsOption = {
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
            color: r.isFiveStar ? (r.isWon === false ? '#D4433B' : theme.gold) : '#e5e7eb'
          }
        })),
        barMaxWidth: 20
      }
    ]
  }

  const chartRef = useECharts(option)

  return (
    <div className="bg-white rounded-xl border border-gray-200 p-4">
      <div className="flex items-center justify-between mb-3">
        <div>
          <div className="text-sm font-semibold text-gray-900">欧非曲线</div>
          <div className="text-xs text-gray-400">金色 = 5⭐ · 红色 = 歪了</div>
        </div>
      </div>
      <div ref={chartRef} className="w-full h-48" />
    </div>
  )
}

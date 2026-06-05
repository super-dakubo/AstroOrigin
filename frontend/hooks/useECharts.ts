import { useEffect, useRef } from 'react'
import { init, type ECharts, type EChartsOption } from '../lib/echarts'

export function useECharts(option: EChartsOption) {
  const chartRef = useRef<HTMLDivElement>(null)
  const instanceRef = useRef<ECharts | null>(null)

  // 初始化图表（仅一次），组件卸载时销毁
  useEffect(() => {
    if (!chartRef.current) return

    instanceRef.current = init(chartRef.current)

    const handleResize = () => instanceRef.current?.resize()
    window.addEventListener('resize', handleResize)

    return () => {
      window.removeEventListener('resize', handleResize)
      instanceRef.current?.dispose()
      instanceRef.current = null
    }
  }, [])

  // 数据变更时只更新配置，不销毁重建
  useEffect(() => {
    instanceRef.current?.setOption(option)
  }, [option])

  return chartRef
}

import { useEffect, useRef } from 'react'
import * as echarts from 'echarts'

export function useECharts(option: echarts.EChartsOption) {
  const chartRef = useRef<HTMLDivElement>(null)
  const instanceRef = useRef<echarts.ECharts | null>(null)

  // 初始化图表（仅一次），组件卸载时销毁
  useEffect(() => {
    if (!chartRef.current) return

    instanceRef.current = echarts.init(chartRef.current)

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

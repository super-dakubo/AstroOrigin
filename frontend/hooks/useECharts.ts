import { useEffect, useRef } from 'react';
import * as echarts from 'echarts';

export function useECharts(option: echarts.EChartsOption) {
  const chartRef = useRef<HTMLDivElement>(null);
  const instanceRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current) return;

    instanceRef.current = echarts.init(chartRef.current);
    instanceRef.current.setOption(option);

    const handleResize = () => instanceRef.current?.resize();
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      instanceRef.current?.dispose();
      instanceRef.current = null;
    };
  }, [option]);

  return chartRef;
}

import type { ReactNode } from 'react';

interface StatCardProps {
  label: string;
  value: string | number;
  sub?: string;
  subColor?: string;
  prefix?: ReactNode;
}

export function StatCard({ label, value, sub, subColor, prefix }: StatCardProps) {
  return (
    <div className="bg-white rounded-xl border border-gray-200 p-4">
      <div className="flex items-center gap-1.5 text-xs text-gray-400 mb-1">
        {prefix}
        {label}
      </div>
      <div className="text-2xl font-bold text-gray-900">{value}</div>
      {sub && (
        <div className="text-xs mt-0.5" style={{ color: subColor ?? 'inherit' }}>
          {sub}
        </div>
      )}
    </div>
  );
}

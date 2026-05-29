interface GachaRecord {
  id: number;
  gameKind: string;
  itemName: string;
  starRating: number;
  recordDate: string;
  isWon: boolean;
}

interface RecordTableProps {
  records: GachaRecord[];
  onDelete?: (id: number) => void;
}

export function RecordTable({ records, onDelete }: RecordTableProps) {
  if (records.length === 0) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">
        暂无记录
      </div>
    );
  }

  return (
    <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
      <div className="grid grid-cols-[1.5fr_3fr_1fr_1fr_48px] gap-2 px-4 py-2.5 bg-gray-50 text-xs font-medium text-gray-400">
        <span>日期</span>
        <span>物品</span>
        <span>星级</span>
        <span />
        <span />
      </div>
      <div className="divide-y divide-gray-100">
        {records.map((r) => (
          <div
            key={r.id}
            className="grid grid-cols-[1.5fr_3fr_1fr_1fr_48px] gap-2 px-4 py-2.5 text-sm items-center"
          >
            <span className="text-gray-400">{r.recordDate}</span>
            <span className="text-gray-900 font-medium">{r.itemName}</span>
            <span className={r.starRating === 5 ? 'text-amber-500 font-semibold' : 'text-gray-300'}>
              {'★'.repeat(r.starRating)}
            </span>
            <span>
              {r.starRating === 5 && !r.isWon && (
                <span className="text-xs text-red-500 font-medium">歪了</span>
              )}
              {r.starRating === 5 && r.isWon && (
                <span className="text-xs text-green-600 font-medium">欧 ✓</span>
              )}
            </span>
            <button
              onClick={() => onDelete?.(r.id)}
              className="text-gray-300 hover:text-red-500 transition-colors text-xs"
              title="删除"
            >
              ✕
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}

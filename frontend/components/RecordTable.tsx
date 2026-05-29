import { useState, useRef, useEffect } from 'react';

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
  onSave?: (id: number, data: { itemName: string; starRating: number; recordDate: string; isWon: boolean }) => void;
}

type EditState = {
  id: number;
  itemName: string;
  starRating: number;
  recordDate: string;
  isWon: boolean;
} | null;

export function RecordTable({ records, onDelete, onSave }: RecordTableProps) {
  const [editing, setEditing] = useState<EditState>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  const startEdit = (r: GachaRecord) => {
    setEditing({
      id: r.id,
      itemName: r.itemName,
      starRating: r.starRating,
      recordDate: r.recordDate,
      isWon: r.isWon,
    });
  };

  const cancelEdit = () => setEditing(null);

  const saveEdit = () => {
    if (!editing) return;
    onSave?.(editing.id, {
      itemName: editing.itemName,
      starRating: editing.starRating,
      recordDate: editing.recordDate,
      isWon: editing.isWon,
    });
    setEditing(null);
  };

  if (records.length === 0) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">
        暂无记录
      </div>
    );
  }

  return (
    <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
      <div className="grid grid-cols-[1.5fr_3fr_80px_80px_48px] gap-2 px-4 py-2.5 bg-gray-50 text-xs font-medium text-gray-400">
        <span>日期</span>
        <span>物品</span>
        <span>星级</span>
        <span />
        <span />
      </div>
      <div className="divide-y divide-gray-100">
        {records.map((r) => {
          const isEditing = editing?.id === r.id;

          if (isEditing) {
            return (
              <div
                key={r.id}
                className="grid grid-cols-[1.5fr_3fr_80px_80px_48px] gap-2 px-4 py-2 text-sm items-center bg-blue-50"
              >
                <input
                  ref={inputRef}
                  className="w-full px-2 py-1 border border-blue-300 rounded text-sm"
                  value={editing.recordDate}
                  onChange={(e) => setEditing({ ...editing, recordDate: e.target.value })}
                  onKeyDown={(e) => e.key === 'Enter' ? saveEdit() : e.key === 'Escape' ? cancelEdit() : undefined}
                />
                <input
                  className="w-full px-2 py-1 border border-blue-300 rounded text-sm"
                  value={editing.itemName}
                  onChange={(e) => setEditing({ ...editing, itemName: e.target.value })}
                  onKeyDown={(e) => e.key === 'Enter' ? saveEdit() : e.key === 'Escape' ? cancelEdit() : undefined}
                />
                <input
                  className="w-16 px-2 py-1 border border-blue-300 rounded text-sm text-center"
                  value={editing.starRating}
                  onChange={(e) => setEditing({ ...editing, starRating: parseInt(e.target.value) || 0 })}
                  onKeyDown={(e) => e.key === 'Enter' ? saveEdit() : e.key === 'Escape' ? cancelEdit() : undefined}
                />
                <label className="flex items-center gap-1 text-xs cursor-pointer">
                  <input
                    type="checkbox"
                    checked={editing.isWon}
                    onChange={(e) => setEditing({ ...editing, isWon: e.target.checked })}
                  />
                  {editing.isWon ? '欧 ✓' : '歪了'}
                </label>
                <div className="flex gap-1">
                  <button onClick={saveEdit} className="text-green-600 hover:text-green-700 text-xs font-bold" title="保存">✓</button>
                  <button onClick={cancelEdit} className="text-gray-400 hover:text-gray-600 text-xs" title="取消">✕</button>
                </div>
              </div>
            );
          }

          return (
            <div
              key={r.id}
              className="grid grid-cols-[1.5fr_3fr_80px_80px_48px] gap-2 px-4 py-2.5 text-sm items-center cursor-default"
              onDoubleClick={() => startEdit(r)}
              title="双击编辑"
            >
              <span className={`${r.recordDate ? 'text-gray-900' : 'text-gray-300 italic'}`}>
                {r.recordDate || '未识别'}
              </span>
              <span className={`font-medium ${
                r.starRating === 5 ? 'text-amber-500' :
                r.starRating === 4 ? 'text-purple-500' :
                r.itemName ? 'text-gray-900' : 'text-gray-300 italic'
              }`}>
                {r.itemName || '未识别'}
              </span>
              <span className={
                r.starRating === 5 ? 'text-amber-500 font-bold' :
                r.starRating === 4 ? 'text-purple-500 font-semibold' :
                r.starRating > 0 ? 'text-gray-400' : 'text-gray-300 italic'
              }>
                {r.starRating > 0 ? '★'.repeat(r.starRating) : '?'}
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
          );
        })}
      </div>
    </div>
  );
}

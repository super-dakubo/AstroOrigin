import { useState, useRef, useEffect } from 'react'
import { Pencil } from 'lucide-react'
import type { GachaRecord } from '../lib/types'

function generatePageNumbers(current: number, total: number): (number | '...')[] {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1)
  }
  const pages: (number | '...')[] = [1]

  if (current - 2 > 2) pages.push('...')

  const start = Math.max(2, current - 2)
  const end = Math.min(total - 1, current + 2)
  for (let i = start; i <= end; i++) pages.push(i)

  if (current + 2 < total - 1) pages.push('...')

  if (total > 1) pages.push(total)
  return pages
}

interface RecordTableProps {
  records: GachaRecord[]
  onDelete?: (id: number) => void
  onSave?: (
    id: number,
    data: {
      itemName: string
      itemType: string
      bannerType: string
      starRating: number
      recordDate: string
      isWon: boolean
    }
  ) => void
  onToggleWon?: (id: number, isWon: boolean) => void
  page: number
  total: number
  pageSize: number
  onPageChange: (page: number) => void
}

type EditState = {
  id: number
  itemName: string
  itemType: string
  bannerType: string
  starRating: number
  recordDate: string
  isWon: boolean
} | null

const TOTAL_PAGES_CAP = 9999

export function RecordTable({
  records,
  onDelete,
  onSave,
  onToggleWon,
  page,
  total,
  pageSize,
  onPageChange
}: RecordTableProps) {
  const [editing, setEditing] = useState<EditState>(null)
  const nameRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (editing) nameRef.current?.focus()
  }, [editing])

  const totalPages = Math.min(Math.ceil(total / pageSize) || 1, TOTAL_PAGES_CAP)

  const startEdit = (r: GachaRecord) => {
    setEditing({
      id: r.id,
      itemName: r.itemName,
      itemType: r.itemType,
      bannerType: r.bannerType,
      starRating: r.starRating,
      recordDate: r.recordDate,
      isWon: r.isWon
    })
  }

  const cancelEdit = () => setEditing(null)

  const saveEdit = () => {
    if (!editing) return
    onSave?.(editing.id, {
      itemName: editing.itemName,
      itemType: editing.itemType,
      bannerType: editing.bannerType,
      starRating: editing.starRating,
      recordDate: editing.recordDate,
      isWon: editing.isWon
    })
    setEditing(null)
  }

  if (records.length === 0) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400">
        暂无记录
      </div>
    )
  }

  return (
    <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
      <table className="w-full">
        <colgroup>
          <col style={{ width: '25%' }} />
          <col style={{ width: '10%' }} />
          <col style={{ width: '13%' }} />
          <col style={{ width: '32%' }} />
          <col style={{ width: '8%' }} />
          <col style={{ width: '8%' }} />
          <col style={{ width: '4%' }} />
        </colgroup>
        <thead>
          <tr className="bg-gray-50 text-xs font-medium text-gray-400">
            <th className="px-4 py-2.5 text-left">日期</th>
            <th className="px-4 py-2.5 text-left">种类</th>
            <th className="px-4 py-2.5 text-left">卡池</th>
            <th className="px-4 py-2.5 text-left">物品</th>
            <th className="px-4 py-2.5 text-left">星级</th>
            <th className="px-4 py-2.5 text-left">结果</th>
            <th className="px-4 py-2.5">
              <span className="sr-only">操作</span>
            </th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-100">
          {records.map((r) => {
            const isEditing = editing?.id === r.id

            if (isEditing) {
              return (
                <tr key={r.id} className="bg-blue-50">
                  <td className="px-4 py-2">
                    <input
                      className="w-full px-2 py-1 border border-blue-300 rounded text-sm"
                      value={editing.recordDate}
                      onChange={(e) => setEditing({ ...editing, recordDate: e.target.value })}
                      onKeyDown={(e) =>
                        e.key === 'Enter'
                          ? saveEdit()
                          : e.key === 'Escape'
                            ? cancelEdit()
                            : undefined
                      }
                    />
                  </td>
                  <td className="px-4 py-2">
                    <select
                      className="w-full px-2 py-1 border border-blue-300 rounded text-sm"
                      value={editing.itemType}
                      onChange={(e) => setEditing({ ...editing, itemType: e.target.value })}
                    >
                      <option value="角色">角色</option>
                      <option value="光锥">光锥</option>
                    </select>
                  </td>
                  <td className="px-4 py-2">
                    <input
                      className="w-full px-2 py-1 border border-blue-300 rounded text-sm"
                      value={editing.bannerType}
                      onChange={(e) => setEditing({ ...editing, bannerType: e.target.value })}
                      onKeyDown={(e) =>
                        e.key === 'Enter'
                          ? saveEdit()
                          : e.key === 'Escape'
                            ? cancelEdit()
                            : undefined
                      }
                    />
                  </td>
                  <td className="px-4 py-2">
                    <input
                      ref={nameRef}
                      className="w-full px-2 py-1 border border-blue-300 rounded text-sm"
                      value={editing.itemName}
                      onChange={(e) => setEditing({ ...editing, itemName: e.target.value })}
                      onKeyDown={(e) =>
                        e.key === 'Enter'
                          ? saveEdit()
                          : e.key === 'Escape'
                            ? cancelEdit()
                            : undefined
                      }
                    />
                  </td>
                  <td className="px-4 py-2">
                    <input
                      className="w-14 px-2 py-1 border border-blue-300 rounded text-sm text-center"
                      value={editing.starRating}
                      onChange={(e) =>
                        setEditing({ ...editing, starRating: parseInt(e.target.value) || 0 })
                      }
                      onKeyDown={(e) =>
                        e.key === 'Enter'
                          ? saveEdit()
                          : e.key === 'Escape'
                            ? cancelEdit()
                            : undefined
                      }
                    />
                  </td>
                  <td className="px-4 py-2">
                    <label className="flex items-center gap-1 text-xs cursor-pointer">
                      <input
                        type="checkbox"
                        checked={editing.isWon}
                        onChange={(e) => setEditing({ ...editing, isWon: e.target.checked })}
                      />
                      {editing.isWon ? '欧' : '歪'}
                    </label>
                  </td>
                  <td className="px-4 py-2">
                    <div className="flex gap-1">
                      <button
                        onClick={saveEdit}
                        className="text-green-600 hover:text-green-700 text-xs font-bold"
                        title="保存"
                      >
                        ✓
                      </button>
                      <button
                        onClick={cancelEdit}
                        className="text-gray-400 hover:text-gray-600 text-xs"
                        title="取消"
                      >
                        ✕
                      </button>
                    </div>
                  </td>
                </tr>
              )
            }

            return (
              <tr key={r.id} className="group">
                <td
                  className={`px-4 py-2.5 ${r.recordDate ? 'text-gray-900' : 'text-gray-300 italic'}`}
                >
                  {r.recordDate || '未识别'}
                </td>
                <td className="px-4 py-2.5 text-gray-500 text-xs">{r.itemType || ''}</td>
                <td className="px-4 py-2.5">
                  {r.bannerType ? (
                    <span
                      className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${
                        r.bannerType.includes('角色')
                          ? 'bg-blue-50 text-blue-600'
                          : r.bannerType.includes('光锥') || r.bannerType.includes('武器')
                            ? 'bg-purple-50 text-purple-600'
                            : 'bg-gray-50 text-gray-500'
                      }`}
                    >
                      {r.bannerType.replace('跃迁', '').replace('祈愿', '')}
                    </span>
                  ) : (
                    ''
                  )}
                </td>
                <td
                  className={`px-4 py-2.5 font-medium ${
                    r.starRating === 5
                      ? 'text-amber-500'
                      : r.starRating === 4
                        ? 'text-purple-500'
                        : r.itemName
                          ? 'text-gray-900'
                          : 'text-gray-300 italic'
                  }`}
                >
                  <span className="inline-flex items-center gap-1.5">
                    {r.itemName || '未识别'}
                    {r.itemName && (
                      <button
                        onClick={() => startEdit(r)}
                        className="opacity-0 group-hover:opacity-40 hover:opacity-100 transition-opacity"
                        title="编辑"
                      >
                        <Pencil className="w-3 h-3" />
                      </button>
                    )}
                  </span>
                </td>
                <td
                  className={`px-4 py-2.5 ${
                    r.starRating === 5
                      ? 'text-amber-500 font-bold'
                      : r.starRating === 4
                        ? 'text-purple-500 font-semibold'
                        : r.starRating > 0
                          ? 'text-gray-400'
                          : 'text-gray-300 italic'
                  }`}
                >
                  {r.starRating > 0 ? '★'.repeat(r.starRating) : '?'}
                </td>
                <td className="px-4 py-2.5">
                  {r.starRating === 5 ? (
                    <button
                      onClick={() => onToggleWon?.(r.id, !r.isWon)}
                      className={`px-2 py-0.5 rounded text-xs font-medium cursor-pointer transition-colors ${
                        r.isWon
                          ? 'bg-green-50 text-green-600 hover:bg-green-100'
                          : 'bg-red-50 text-red-500 hover:bg-red-100'
                      }`}
                    >
                      {r.isWon ? '欧 ✓' : '歪了'}
                    </button>
                  ) : (
                    <span className="text-gray-300 text-xs">-</span>
                  )}
                </td>
                <td className="px-4 py-2.5">
                  <button
                    onClick={() => onDelete?.(r.id)}
                    className="text-gray-300 hover:text-red-500 transition-colors text-xs opacity-0 group-hover:opacity-100"
                    title="删除"
                  >
                    ✕
                  </button>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>

      {/* 分页 */}
      <div className="flex items-center justify-between px-4 py-2.5 border-t border-gray-100 text-sm text-gray-500">
        <span>
          共 {total} 条 / {totalPages} 页
        </span>
        <div className="flex items-center gap-1">
          <button
            onClick={() => onPageChange(page - 1)}
            disabled={page <= 1}
            className="px-2.5 py-1 rounded border border-gray-200 disabled:opacity-30 hover:bg-gray-50 text-xs"
          >
            上一页
          </button>
          {generatePageNumbers(page, totalPages).map((n, i) =>
            n === '...' ? (
              <span key={`e${i}`} className="px-1 text-gray-300 text-xs">
                ...
              </span>
            ) : (
              <button
                key={n}
                onClick={() => onPageChange(n as number)}
                className={`w-7 h-7 rounded text-xs ${
                  page === n ? 'bg-blue-500 text-white' : 'border border-gray-200 hover:bg-gray-50'
                }`}
              >
                {n}
              </button>
            )
          )}
          <button
            onClick={() => onPageChange(page + 1)}
            disabled={page >= totalPages}
            className="px-2.5 py-1 rounded border border-gray-200 disabled:opacity-30 hover:bg-gray-50 text-xs"
          >
            下一页
          </button>
        </div>
      </div>
    </div>
  )
}

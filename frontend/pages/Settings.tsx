import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { GachaConfig, GameConfig } from '../lib/types'

const GAME_LABELS: Record<string, string> = {
  genshin: '原神',
  starrail: '崩坏：星穹铁道'
}

export function Settings() {
  const [config, setConfig] = useState<GachaConfig | null>(null)
  const [dirty, setDirty] = useState(false)
  const [msg, setMsg] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    invoke<GachaConfig>('get_gacha_config')
      .then((c) => {
        setConfig(c)
        setLoading(false)
      })
      .catch((e) => {
        setMsg(`加载配置失败: ${e}`)
        setLoading(false)
      })
  }, [])

  const updateGame = (game: string, patch: Partial<GameConfig>) => {
    if (!config) return
    setConfig({
      ...config,
      games: { ...config.games, [game]: { ...config.games[game], ...patch } }
    })
    setDirty(true)
  }

  const handleSave = async () => {
    if (!config) return
    try {
      await invoke('save_gacha_config', { config })
      setDirty(false)
      setMsg('保存成功')
      setTimeout(() => setMsg(null), 2500)
    } catch (e) {
      setMsg(`保存失败: ${e}`)
    }
  }

  const handleReset = async (game: string) => {
    try {
      const c = await invoke<GachaConfig>('reset_gacha_config', { game })
      setConfig(c)
      setDirty(true)
      setMsg(`「${GAME_LABELS[game]}」已恢复默认`)
      setTimeout(() => setMsg(null), 2500)
    } catch (e) {
      setMsg(`重置失败: ${e}`)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48">
        <p className="text-gray-400 text-sm">加载中...</p>
      </div>
    )
  }

  if (!config) {
    return <p className="text-red-500 text-sm">配置加载失败</p>
  }

  return (
    <div className="max-w-2xl mx-auto space-y-4">
      <h1 className="text-2xl font-bold text-gray-900">设置</h1>
      <p className="text-sm text-gray-500">修改抽卡记录自动导入的配置项</p>

      {Object.entries(config.games).map(([game, gc]) => (
        <GameConfigSection
          key={game}
          label={GAME_LABELS[game] ?? game}
          config={gc}
          onChange={(patch) => updateGame(game, patch)}
          onReset={() => handleReset(game)}
        />
      ))}

      <div className="flex items-center gap-3 pt-2">
        <button
          onClick={handleSave}
          disabled={!dirty}
          className="px-5 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
        >
          保存设置
        </button>
        {msg && <span className="text-sm text-green-600">{msg}</span>}
      </div>
    </div>
  )
}

function GameConfigSection({
  label,
  config,
  onChange,
  onReset
}: {
  label: string
  config: GameConfig
  onChange: (patch: Partial<GameConfig>) => void
  onReset: () => void
}) {
  const [expanded, setExpanded] = useState(true)

  return (
    <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between px-4 py-3 text-sm font-medium text-gray-900 hover:bg-gray-50 transition-colors"
      >
        {label}
        <span className="text-gray-400 text-xs">{expanded ? '▼' : '▶'}</span>
      </button>

      {expanded && (
        <div className="px-4 pb-4 space-y-3 border-t border-gray-100 pt-3">
          {/* 日志目录 */}
          <div>
            <label className="text-xs text-gray-500 block mb-1">日志目录（每行一个）</label>
            <textarea
              className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm font-mono resize-y"
              rows={config.logDirs.length + 1}
              value={config.logDirs.join('\n')}
              onChange={(e) =>
                onChange({
                  logDirs: e.target.value
                    .split('\n')
                    .map((s) => s.trim())
                    .filter(Boolean)
                })
              }
            />
          </div>

          {/* API 地址 */}
          <div>
            <label className="text-xs text-gray-500 block mb-1">API 地址</label>
            <input
              className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm font-mono"
              value={config.apiUrl}
              onChange={(e) => onChange({ apiUrl: e.target.value })}
            />
          </div>

          {/* 卡池类型 */}
          <div>
            <label className="text-xs text-gray-500 block mb-1">卡池类型映射（只读参考）</label>
            <div className="flex flex-wrap gap-2">
              {Object.entries(config.gachaTypes).map(([code, name]) => (
                <span key={code} className="px-2 py-1 text-xs bg-gray-100 rounded-md text-gray-600">
                  {code} → {name}
                </span>
              ))}
            </div>
          </div>

          <button
            onClick={onReset}
            className="text-xs text-gray-400 hover:text-red-500 transition-colors"
          >
            ↺ 恢复默认
          </button>
        </div>
      )}
    </div>
  )
}

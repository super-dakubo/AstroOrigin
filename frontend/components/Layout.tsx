import { useEffect, type ReactNode } from 'react'
import { NavLink } from 'react-router-dom'
import { ROUTES } from '../lib/constants'
import { GameSwitch } from './GameSwitch'
import { useGameStore } from '../stores/gameStore'

const navItems = [
  { path: ROUTES.OVERVIEW, label: '总览' },
  { path: ROUTES.GACHA, label: '抽卡记录' },
  { path: ROUTES.PLAYTIME, label: '游戏时长' },
  { path: ROUTES.SCREENSHOTS, label: '截图' }
]

export function Layout({ children }: { children: ReactNode }) {
  const theme = useGameStore((s) => s.theme)

  // Ensure dark class is always removed (both themes are light)
  useEffect(() => {
    document.documentElement.classList.remove('dark')
  }, [])

  const barStyle = { background: theme.barGradient }

  return (
    <div
      className="min-h-screen flex flex-col transition-colors duration-300"
      style={{ background: theme.bg }}
    >
      {/* Top accent bar */}
      <div className="h-0.5" style={barStyle} />

      {/* Navigation */}
      <nav className="sticky top-0 z-50 bg-white/80 backdrop-blur-md border-b border-gray-200/60 px-6">
        <div className="max-w-6xl mx-auto h-14 flex items-center gap-6">
          <span className="text-base font-bold text-gray-900 whitespace-nowrap">
            {theme.appName}
          </span>
          <div className="flex items-center gap-1 flex-1">
            {navItems.map((item) => (
              <NavLink
                key={item.path}
                to={item.path}
                className={({ isActive }) =>
                  `px-3 py-1.5 text-sm rounded-lg transition-colors ${
                    isActive
                      ? 'text-gray-900 font-medium bg-gray-100'
                      : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
                  }`
                }
              >
                {item.label}
              </NavLink>
            ))}
          </div>
          <NavLink
            to={ROUTES.SETTINGS}
            className={({ isActive }) =>
              `p-2 rounded-lg transition-colors ${
                isActive
                  ? 'text-gray-900 bg-gray-100'
                  : 'text-gray-400 hover:text-gray-600 hover:bg-gray-50'
              }`
            }
            title="设置"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </NavLink>
          <GameSwitch />
        </div>
      </nav>

      {/* Content */}
      <main className="flex-1 max-w-6xl mx-auto w-full px-6 py-6">{children}</main>
    </div>
  )
}

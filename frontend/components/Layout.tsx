import React from 'react';
import { NavLink } from 'react-router-dom';
import { ROUTES } from '../lib/constants';
import { GameSwitch } from './GameSwitch';
import { useGameStore } from '../stores/gameStore';

const navItems = [
  { path: ROUTES.OVERVIEW, label: '总览' },
  { path: ROUTES.GACHA, label: '抽卡记录' },
  { path: ROUTES.PLAYTIME, label: '游戏时长' },
  { path: ROUTES.SCREENSHOTS, label: '截图' },
];

export function Layout({ children }: { children: React.ReactNode }) {
  const theme = useGameStore((s) => s.theme);

  // Ensure dark class is always removed (both themes are light)
  React.useEffect(() => {
    document.documentElement.classList.remove('dark');
  }, []);

  const barStyle = { background: theme.barGradient };

  return (
    <div className="min-h-screen flex flex-col" style={{ background: theme.bg }}>
      {/* Top accent bar */}
      <div className="h-0.5" style={barStyle} />

      {/* Navigation */}
      <nav className="sticky top-0 z-50 bg-white/80 backdrop-blur-md border-b border-gray-200/60 px-6">
        <div className="max-w-6xl mx-auto h-14 flex items-center gap-6">
          <span className="text-base font-bold text-gray-900 whitespace-nowrap">{theme.appName}</span>
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
          <GameSwitch />
        </div>
      </nav>

      {/* Content */}
      <main className="flex-1 max-w-6xl mx-auto w-full px-6 py-6">
        {children}
      </main>
    </div>
  );
}

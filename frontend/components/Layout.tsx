import React from 'react';
import { NavLink } from 'react-router-dom';
import { ROUTES } from '../lib/constants';
import { GameSwitch } from './GameSwitch';
import { useGameStore } from '../stores/gameStore';
import { CalendarDays, ChartBar, Clock, ScanSearch } from 'lucide-react';

const navItems = [
  { path: ROUTES.OVERVIEW, label: '总览', icon: ChartBar },
  { path: ROUTES.GACHA, label: '抽卡记录', icon: CalendarDays },
  { path: ROUTES.PLAYTIME, label: '游戏时长', icon: Clock },
  { path: ROUTES.SCREENSHOTS, label: '截图', icon: ScanSearch },
];

export function Layout({ children }: { children: React.ReactNode }) {
  const { theme, currentGame } = useGameStore();

  // Toggle dark class on <html> for HeroUI dark mode
  React.useEffect(() => {
    document.documentElement.classList.toggle('dark', theme.isDark);
  }, [theme.isDark]);

  const barStyle = { background: theme.barGradient };
  const textClass = currentGame === 'starrail' ? 'text-white' : 'text-gray-900';
  const navBg = currentGame === 'starrail'
    ? 'bg-slate-900/90 backdrop-blur-md border-b border-slate-800/60'
    : 'bg-white/80 backdrop-blur-md border-b border-gray-200/60';

  return (
    <div className="min-h-screen flex flex-col" style={{ background: theme.bg }}>
      {/* Top accent bar */}
      <div className="h-0.5" style={barStyle} />

      {/* Navigation */}
      <nav className={`sticky top-0 z-50 ${navBg} px-6`}>
        <div className="max-w-6xl mx-auto h-14 flex items-center gap-6">
          <span className={`text-base font-bold whitespace-nowrap ${textClass}`}>{theme.appName}</span>
          <div className="flex items-center gap-1 flex-1">
            {navItems.map((item) => {
              const isDark = currentGame === 'starrail';
              return (
                <NavLink
                  key={item.path}
                  to={item.path}
                  className={({ isActive }) =>
                    `flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg transition-colors ${
                      isActive
                        ? isDark
                          ? 'text-white font-medium bg-white/10'
                          : 'text-gray-900 font-medium bg-gray-100'
                        : isDark
                          ? 'text-slate-400 hover:text-slate-200 hover:bg-white/5'
                          : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
                    }`
                  }
                >
                  <item.icon className="w-4 h-4" />
                  {item.label}
                </NavLink>
              );
            })}
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

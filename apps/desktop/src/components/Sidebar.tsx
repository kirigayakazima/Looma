import React from 'react';
import {
  LayoutDashboard,
  FolderArchive,
  Boxes,
  BookmarkCheck,
  Clock,
  BookOpen,
  Sparkles,
  Settings,
  HardDrive
} from 'lucide-react';
import { useI18n } from '../i18n';

export type NavTab = 'home' | 'assets' | 'entities' | 'collections' | 'timeline' | 'memories' | 'insights' | 'settings';

interface SidebarProps {
  currentTab: NavTab;
  onSelectTab: (tab: NavTab) => void;
  vaultName: string;
}

export const Sidebar: React.FC<SidebarProps> = ({ currentTab, onSelectTab, vaultName }) => {
  const { t } = useI18n();

  const navItems: { id: NavTab; label: string; icon: React.ReactNode }[] = [
    { id: 'home', label: t.nav.home, icon: <LayoutDashboard className="w-4 h-4" /> },
    { id: 'assets', label: t.nav.assets, icon: <FolderArchive className="w-4 h-4" /> },
    { id: 'entities', label: t.nav.entities, icon: <Boxes className="w-4 h-4" /> },
    { id: 'collections', label: t.nav.collections, icon: <BookmarkCheck className="w-4 h-4" /> },
    { id: 'timeline', label: t.nav.timeline, icon: <Clock className="w-4 h-4" /> },
    { id: 'memories', label: t.nav.memories, icon: <BookOpen className="w-4 h-4" /> },
    { id: 'insights', label: t.nav.insights, icon: <Sparkles className="w-4 h-4 text-amber-500" /> },
    { id: 'settings', label: t.nav.settings, icon: <Settings className="w-4 h-4" /> },
  ];

  return (
    <aside className="w-64 bg-neutral-900 border-r border-neutral-800 flex flex-col justify-between h-screen select-none">
      <div>
        {/* App Title & Vault Info */}
        <div className="px-5 py-5 border-b border-neutral-800 flex items-center space-x-3">
          <div className="w-8 h-8 rounded-lg bg-indigo-600 flex items-center justify-center font-bold text-white shadow-sm shadow-indigo-500/20">
            L
          </div>
          <div>
            <h1 className="text-sm font-semibold tracking-wide text-neutral-100">Looma</h1>
            <p className="text-xs text-neutral-400 truncate max-w-[150px]">{vaultName || t.home.defaultVaultName}</p>
          </div>
        </div>

        {/* Navigation list */}
        <nav className="p-3 space-y-1">
          {navItems.map((item) => {
            const active = currentTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                className={`w-full flex items-center space-x-3 px-3 py-2 rounded-md text-sm transition-colors ${
                  active
                    ? 'bg-neutral-800 text-white font-medium shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50'
                }`}
              >
                <span className={active ? 'text-indigo-400' : 'text-neutral-400'}>{item.icon}</span>
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Vault Footer status */}
      <div className="p-4 border-t border-neutral-800 text-xs text-neutral-400 flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <HardDrive className="w-3.5 h-3.5 text-neutral-500" />
          <span>{t.common.localFirst}</span>
        </div>
        <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/10 text-emerald-400 font-mono">
          {t.common.ready}
        </span>
      </div>
    </aside>
  );
};

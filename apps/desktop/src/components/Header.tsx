import React from 'react';
import { Search, Database } from 'lucide-react';
import { VaultInfo } from '../types';
import { useI18n } from '../i18n';

interface HeaderProps {
  vaultInfo: VaultInfo | null;
  currentTitle: string;
  onOpenSearch?: () => void;
}

export const Header: React.FC<HeaderProps> = ({ vaultInfo, currentTitle, onOpenSearch }) => {
  const { t } = useI18n();

  return (
    <header className="h-14 border-b border-neutral-800 bg-neutral-900/50 backdrop-blur px-6 flex items-center justify-between">
      <div className="flex items-center space-x-3">
        <h2 className="text-sm font-medium text-neutral-200">{currentTitle}</h2>
      </div>

      <div className="flex items-center space-x-4">
        {/* Global Search shortcut (Ctrl+K) */}
        <div
          onClick={onOpenSearch}
          className="relative w-80 cursor-pointer group"
        >
          <Search className="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-neutral-400 group-hover:text-indigo-400 transition-colors" />
          <input
            type="text"
            readOnly
            onClick={onOpenSearch}
            placeholder={t.header.searchPlaceholder}
            className="w-full bg-neutral-800/80 border border-neutral-700/60 rounded-md pl-9 pr-12 py-1.5 text-xs text-neutral-200 placeholder-neutral-500 cursor-pointer focus:outline-none group-hover:border-neutral-600 transition-colors"
          />
          <kbd className="absolute right-2.5 top-1/2 -translate-y-1/2 text-[10px] text-neutral-400 bg-neutral-700/50 px-1.5 py-0.5 rounded font-mono border border-neutral-600/40">
            {t.header.searchShortcut}
          </kbd>
        </div>

        {/* Database indicator */}
        <div className="flex items-center space-x-1.5 text-xs text-neutral-400 bg-neutral-800/50 px-2.5 py-1 rounded border border-neutral-700/40">
          <Database className="w-3 h-3 text-indigo-400" />
          <span>{t.header.versionPrefix}{vaultInfo?.version || '0.2'}</span>
        </div>
      </div>
    </header>
  );
};

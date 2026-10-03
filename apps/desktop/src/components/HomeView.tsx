import React from 'react';
import { VaultInfo, VaultStats } from '../types';
import { FolderArchive, Boxes, BookmarkCheck, BookOpen, ShieldCheck, Cpu } from 'lucide-react';
import { useI18n } from '../i18n';

interface HomeViewProps {
  vaultInfo: VaultInfo | null;
  vaultStats: VaultStats | null;
  onNavigate: (tab: 'assets' | 'entities' | 'memories' | 'collections') => void;
}

export const HomeView: React.FC<HomeViewProps> = ({ vaultInfo, vaultStats, onNavigate }) => {
  const { t } = useI18n();

  const statCards = [
    {
      title: t.home.statAssets,
      count: vaultStats?.total_assets ?? 0,
      description: t.home.statAssetsDesc,
      icon: <FolderArchive className="w-5 h-5 text-blue-400" />,
      onClick: () => onNavigate('assets'),
    },
    {
      title: t.home.statEntities,
      count: vaultStats?.total_entities ?? 0,
      description: t.home.statEntitiesDesc,
      icon: <Boxes className="w-5 h-5 text-indigo-400" />,
      onClick: () => onNavigate('entities'),
    },
    {
      title: t.home.statCollections,
      count: vaultStats?.total_collections ?? 0,
      description: t.home.statCollectionsDesc,
      icon: <BookmarkCheck className="w-5 h-5 text-emerald-400" />,
      onClick: () => onNavigate('collections'),
    },
    {
      title: t.home.statMemories,
      count: vaultStats?.total_memories ?? 0,
      description: t.home.statMemoriesDesc,
      icon: <BookOpen className="w-5 h-5 text-amber-400" />,
      onClick: () => onNavigate('memories'),
    },
  ];

  return (
    <div className="space-y-6 max-w-5xl">
      {/* Welcome Banner */}
      <div className="p-6 rounded-xl bg-gradient-to-r from-neutral-900 to-neutral-800/80 border border-neutral-800">
        <h2 className="text-xl font-semibold text-white mb-1">
          {vaultInfo?.name || t.home.defaultVaultName}
        </h2>
        <p className="text-sm text-neutral-400 leading-relaxed max-w-2xl">
          {t.home.bannerSubtitle}
        </p>
        <div className="mt-4 flex items-center space-x-4 text-xs text-neutral-400">
          <span className="flex items-center space-x-1.5">
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
            <span>{t.home.offlineBadge}</span>
          </span>
          <span className="flex items-center space-x-1.5">
            <Cpu className="w-4 h-4 text-indigo-400" />
            <span>{t.home.engineBadge} v{vaultInfo?.version || '0.2.0'}</span>
          </span>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {statCards.map((card, idx) => (
          <button
            key={idx}
            onClick={card.onClick}
            className="p-4 rounded-lg bg-neutral-900 border border-neutral-800 hover:border-neutral-700 hover:bg-neutral-850 text-left transition-all group"
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-neutral-400">{card.title}</span>
              {card.icon}
            </div>
            <div className="text-2xl font-bold text-neutral-100 group-hover:text-indigo-400 transition-colors">
              {card.count}
            </div>
            <div className="text-[11px] text-neutral-500 mt-1 truncate">{card.description}</div>
          </button>
        ))}
      </div>

      {/* System Foundation Info */}
      <div className="p-5 rounded-lg bg-neutral-900/60 border border-neutral-800">
        <h3 className="text-sm font-medium text-neutral-200 mb-3">{t.home.systemStatusTitle}</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
          <div className="space-y-2">
            <div className="flex justify-between py-1 border-b border-neutral-800/60">
              <span className="text-neutral-400">{t.home.vaultDir}</span>
              <span className="text-neutral-200 font-mono text-[11px] truncate max-w-[240px]">
                {vaultInfo?.vault_path || t.home.defaultStorage}
              </span>
            </div>
            <div className="flex justify-between py-1 border-b border-neutral-800/60">
              <span className="text-neutral-400">{t.home.databaseEngine}</span>
              <span className="text-neutral-200 font-mono text-[11px] truncate max-w-[240px]">
                {t.home.databaseEngineDesc}
              </span>
            </div>
          </div>
          <div className="space-y-2">
            <div className="flex justify-between py-1 border-b border-neutral-800/60">
              <span className="text-neutral-400">{t.home.databaseSize}</span>
              <span className="text-neutral-200 font-mono text-[11px]">
                {vaultStats ? (vaultStats.database_size_bytes / 1024).toFixed(1) + ' KB' : '0 KB'}
              </span>
            </div>
            <div className="flex justify-between py-1 border-b border-neutral-800/60">
              <span className="text-neutral-400">{t.home.archPhase}</span>
              <span className="text-indigo-400 font-medium">{t.home.archPhaseVal}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

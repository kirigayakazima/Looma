import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Sidebar, NavTab } from './components/Sidebar';
import { Header } from './components/Header';
import { HomeView } from './components/HomeView';
import { WorksView } from './components/WorksView';
import { AssetsView } from './components/AssetsView';
import { EntitiesView } from './components/EntitiesView';
import { MemoriesView } from './components/MemoriesView';
import { CollectionsView } from './components/CollectionsView';
import { TimelineView } from './components/TimelineView';
import { InsightsView } from './components/InsightsView';
import { SettingsView } from './components/SettingsView';
import { GlobalSearchModal } from './components/GlobalSearchModal';
import { VaultInfo, VaultStats, Asset, Entity, Memory, Collection } from './types';
import { useI18n } from './i18n';
import { LandingView } from './components/LandingView';
import { isTauriEnvironment, webVault } from './webAdapter';

export const App: React.FC = () => {
  const { t } = useI18n();
  const [viewMode, setViewMode] = useState<'landing' | 'app'>(() =>
    isTauriEnvironment() ? 'app' : 'landing'
  );
  const [currentTab, setCurrentTab] = useState<NavTab>('home');
  const [vaultInfo, setVaultInfo] = useState<VaultInfo | null>(null);
  const [vaultStats, setVaultStats] = useState<VaultStats | null>(null);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [entities, setEntities] = useState<Entity[]>([]);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [loading, setLoading] = useState(true);
  const [isSearchOpen, setIsSearchOpen] = useState(false);

  // Safe invoke wrapper with mock fallback for web development preview
  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed or running in web preview:`, err);
      return null;
    }
  };

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const info = await safeInvoke<VaultInfo>('get_vault_info');
      if (info) {
        setVaultInfo(info);
      } else {
        // Fallback mock info if accessed outside Tauri webview
        setVaultInfo({
          name: t.home.defaultVaultName,
          version: '0.2.0',
          vault_path: 'D:/CodePackage/Persoanl/LoomaProject/vault',
          database_path: 'D:/CodePackage/Persoanl/LoomaProject/vault/.looma/vault.db',
          is_initialized: true,
          created_at: new Date().toISOString(),
        });
      }

      const stats = await safeInvoke<VaultStats>('get_vault_stats');
      if (stats) setVaultStats(stats);

      const assetList = await safeInvoke<Asset[]>('list_assets', { limit: 50, offset: 0 });
      if (assetList) setAssets(assetList);

      const entityList = await safeInvoke<Entity[]>('list_entities', {});
      if (entityList) setEntities(entityList);

      const memoryList = await safeInvoke<Memory[]>('list_memories', { limit: 50, offset: 0 });
      if (memoryList) setMemories(memoryList);

      const colList = await safeInvoke<Collection[]>('list_collections');
      if (colList) setCollections(colList);
    } finally {
      setLoading(false);
    }
  }, [t.home.defaultVaultName]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Global Ctrl+K / Cmd+K listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setIsSearchOpen((prev) => !prev);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const getTitle = () => {
    switch (currentTab) {
      case 'home':
        return t.nav.home;
      case 'works':
        return t.nav.works;
      case 'assets':
        return t.nav.assets;
      case 'entities':
        return t.nav.entities;
      case 'collections':
        return t.nav.collections;
      case 'timeline':
        return t.nav.timeline;
      case 'memories':
        return t.nav.memories;
      case 'insights':
        return t.nav.insights;
      case 'settings':
        return t.nav.settings;
    }
  };

  if (viewMode === 'landing' && !isTauriEnvironment()) {
    return <LandingView onEnterDemo={() => setViewMode('app')} />;
  }

  const isWebMode = !isTauriEnvironment();

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-neutral-950 text-neutral-100">
      {/* Top Banner when running in Web Demo Mode */}
      {isWebMode && (
        <div className="bg-neutral-900 border-b border-neutral-800 px-4 py-2 flex items-center justify-between text-xs text-neutral-300 shrink-0 z-40">
          <div className="flex items-center gap-2">
            <span className="flex h-2 w-2 rounded-full bg-emerald-400 animate-pulse" />
            <span className="font-semibold text-emerald-400">Looma Web 演练模式</span>
            <span className="hidden md:inline text-neutral-600">|</span>
            <span className="hidden md:inline text-neutral-400">
              数据保存在当前浏览器本地 (LocalStorage)，支持完整的新建、微调进度、手记与时光轴交互
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                if (window.confirm('确认重置所有演示数据为初始状态吗？')) {
                  webVault.resetToDefault();
                  loadData();
                }
              }}
              className="px-2.5 py-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs transition-colors cursor-pointer"
              title="恢复初始预置的番剧、游戏、手记和图谱数据"
            >
              重置演练数据
            </button>
            <button
              onClick={() => setViewMode('landing')}
              className="px-2.5 py-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium transition-colors cursor-pointer"
            >
              返回官网
            </button>
            <a
              href="https://github.com/kirigayakazima/Looma/releases"
              target="_blank"
              rel="noopener noreferrer"
              className="px-2.5 py-1 rounded bg-emerald-500 hover:bg-emerald-400 text-neutral-950 text-xs font-semibold flex items-center gap-1 transition-colors cursor-pointer shadow-sm shadow-emerald-500/20"
            >
              <span>下载客户端</span>
            </a>
          </div>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          currentTab={currentTab}
          onSelectTab={setCurrentTab}
          vaultName={vaultInfo?.name || t.home.defaultVaultName}
        />

        <div className="flex-1 flex flex-col h-full overflow-hidden">
          <Header
            vaultInfo={vaultInfo}
            currentTitle={getTitle()}
            onOpenSearch={() => setIsSearchOpen(true)}
          />

          <main className="flex-1 overflow-y-auto p-6">
          {loading && !vaultInfo ? (
            <div className="flex items-center justify-center h-full text-neutral-500 text-xs">
              {t.common.initializing}
            </div>
          ) : (
            <>
              {currentTab === 'home' && (
                <HomeView
                  vaultInfo={vaultInfo}
                  vaultStats={vaultStats}
                  onNavigate={(tab) => setCurrentTab(tab)}
                />
              )}
              {currentTab === 'works' && (
                <WorksView
                  assets={assets}
                  onRefreshAll={loadData}
                />
              )}
              {currentTab === 'assets' && (
                <AssetsView assets={assets} entities={entities} onRefresh={loadData} />
              )}
              {currentTab === 'entities' && (
                <EntitiesView entities={entities} assets={assets} onRefresh={loadData} />
              )}
              {currentTab === 'memories' && (
                <MemoriesView
                  memories={memories}
                  entities={entities}
                  assets={assets}
                  onRefresh={loadData}
                />
              )}
              {currentTab === 'collections' && (
                <CollectionsView
                  collections={collections}
                  assets={assets}
                  entities={entities}
                  onRefresh={loadData}
                />
              )}
              {currentTab === 'timeline' && (
                <TimelineView
                  assets={assets}
                  entities={entities}
                  memories={memories}
                  onRefresh={loadData}
                  onNavigateTab={(tab) => setCurrentTab(tab)}
                />
              )}
              {currentTab === 'insights' && (
                <InsightsView onRefreshAll={loadData} />
              )}
              {currentTab === 'settings' && (
                <SettingsView vaultInfo={vaultInfo} onRefresh={loadData} />
              )}
            </>
          )}
        </main>
      </div>
    </div>

      {/* Global Quick Search Modal */}
      <GlobalSearchModal
        isOpen={isSearchOpen}
        onClose={() => setIsSearchOpen(false)}
        assets={assets}
        entities={entities}
        collections={collections}
        memories={memories}
        onNavigate={(tab) => {
          setCurrentTab(tab);
          setIsSearchOpen(false);
        }}
      />
    </div>
  );
};
export default App;

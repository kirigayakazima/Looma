import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Sidebar, NavTab } from './components/Sidebar';
import { Header } from './components/Header';
import { HomeView } from './components/HomeView';
import { AssetsView } from './components/AssetsView';
import { EntitiesView } from './components/EntitiesView';
import { MemoriesView } from './components/MemoriesView';
import { CollectionsView } from './components/CollectionsView';
import { TimelineView } from './components/TimelineView';
import { SettingsView } from './components/SettingsView';
import { GlobalSearchModal } from './components/GlobalSearchModal';
import { VaultInfo, VaultStats, Asset, Entity, Memory, Collection } from './types';
import { useI18n } from './i18n';

export const App: React.FC = () => {
  const { t } = useI18n();
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
      case 'settings':
        return t.nav.settings;
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-neutral-950 text-neutral-100">
      <Sidebar
        currentTab={currentTab}
        onSelectTab={setCurrentTab}
        vaultName={vaultInfo?.name || t.home.defaultVaultName}
      />

      <div className="flex-1 flex flex-col h-screen overflow-hidden">
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
              {currentTab === 'settings' && (
                <SettingsView vaultInfo={vaultInfo} onRefresh={loadData} />
              )}
            </>
          )}
        </main>
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

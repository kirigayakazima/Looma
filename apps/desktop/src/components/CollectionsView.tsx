import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  BookmarkCheck,
  FolderHeart,
  Plus,
  Trash2,
  X,
  Boxes,
  ExternalLink,
  Tag,
  Search,
  Image as ImageIcon,
  FileText,
  Video,
  Music,
  Archive,
  Code,
  Box,
  FileQuestion,
  Calendar,
} from 'lucide-react';
import { Collection, CollectionItem, Asset, Entity } from '../types';
import { useI18n } from '../i18n';

interface CollectionsViewProps {
  collections: Collection[];
  assets: Asset[];
  entities: Entity[];
  onRefresh: () => Promise<void> | void;
}

export const CollectionsView: React.FC<CollectionsViewProps> = ({
  collections,
  assets,
  entities,
  onRefresh,
}) => {
  const { t } = useI18n();

  // Create Collection modal state
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [newDesc, setNewDesc] = useState('');
  const [creating, setCreating] = useState(false);

  // Active Collection detail state
  const [activeCollection, setActiveCollection] = useState<Collection | null>(null);
  const [collectionItems, setCollectionItems] = useState<CollectionItem[]>([]);
  const [loadingItems, setLoadingItems] = useState(false);

  // Add Item to Collection modal state
  const [showAddItemModal, setShowAddItemModal] = useState(false);
  const [addItemTab, setAddItemTab] = useState<'asset' | 'entity'>('asset');
  const [itemSearchQuery, setItemSearchQuery] = useState('');

  // Safe invoke helper
  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  // Load items for active collection
  const loadCollectionItems = useCallback(async (colId: string) => {
    setLoadingItems(true);
    try {
      const items = await safeInvoke<CollectionItem[]>('list_collection_items', {
        collectionId: colId,
      });
      setCollectionItems(items || []);
    } finally {
      setLoadingItems(false);
    }
  }, []);

  useEffect(() => {
    if (activeCollection) {
      loadCollectionItems(activeCollection.id);
    } else {
      setCollectionItems([]);
    }
  }, [activeCollection, loadCollectionItems]);

  // Create new collection
  const handleCreateCollection = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;
    setCreating(true);

    try {
      const newCol: Collection = {
        id: 'col_' + Math.random().toString(36).substring(2, 9),
        title: newTitle.trim(),
        description: newDesc.trim() || null,
        query: null,
        metadata: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      await safeInvoke('create_collection', { collection: newCol });
      setNewTitle('');
      setNewDesc('');
      setShowCreateModal(false);
      await onRefresh();
    } finally {
      setCreating(false);
    }
  };

  // Delete collection
  const handleDeleteCollection = async (id: string, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    if (!window.confirm(t.collections.deleteConfirm)) return;
    await safeInvoke('delete_collection', { id });
    if (activeCollection?.id === id) {
      setActiveCollection(null);
    }
    await onRefresh();
  };

  // Add item to active collection
  const handleAddItem = async (itemId: string, itemType: string) => {
    if (!activeCollection) return;
    await safeInvoke('add_item_to_collection', {
      collectionId: activeCollection.id,
      itemId,
      itemType,
    });
    await loadCollectionItems(activeCollection.id);
    setShowAddItemModal(false);
  };

  // Remove item from active collection
  const handleRemoveItem = async (itemId: string) => {
    if (!activeCollection) return;
    await safeInvoke('remove_item_from_collection', {
      collectionId: activeCollection.id,
      itemId,
    });
    await loadCollectionItems(activeCollection.id);
  };

  // Reveal in explorer
  const handleReveal = async (path: string | null) => {
    if (!path) return;
    await safeInvoke('open_in_file_manager', { path });
  };

  // Map collection items to actual Asset and Entity objects
  const resolvedItems = useMemo(() => {
    return collectionItems.map((ci) => {
      if (ci.item_type === 'asset') {
        const asset = assets.find((a) => a.id === ci.item_id);
        return { type: 'asset' as const, raw: ci, data: asset };
      } else {
        const entity = entities.find((e) => e.id === ci.item_id);
        return { type: 'entity' as const, raw: ci, data: entity };
      }
    });
  }, [collectionItems, assets, entities]);

  // Available items to add (exclude items already in collection)
  const availableAssetsToAdd = useMemo(() => {
    const containedIds = new Set(collectionItems.map((ci) => ci.item_id));
    return assets
      .filter((a) => !containedIds.has(a.id))
      .filter((a) => {
        if (!itemSearchQuery.trim()) return true;
        const q = itemSearchQuery.toLowerCase();
        return (a.path || '').toLowerCase().includes(q) || a.id.includes(q);
      });
  }, [assets, collectionItems, itemSearchQuery]);

  const availableEntitiesToAdd = useMemo(() => {
    const containedIds = new Set(collectionItems.map((ci) => ci.item_id));
    return entities
      .filter((e) => !containedIds.has(e.id))
      .filter((e) => {
        if (!itemSearchQuery.trim()) return true;
        const q = itemSearchQuery.toLowerCase();
        return e.title.toLowerCase().includes(q) || e.entity_type.toLowerCase().includes(q);
      });
  }, [entities, collectionItems, itemSearchQuery]);

  // Format file size
  const formatSize = (bytes: number | null) => {
    if (!bytes) return '—';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  // Kind icon helper
  const getKindIcon = (kind: string) => {
    switch (kind) {
      case 'image':
        return <ImageIcon className="w-3.5 h-3.5 text-sky-400" />;
      case 'video':
        return <Video className="w-3.5 h-3.5 text-purple-400" />;
      case 'audio':
        return <Music className="w-3.5 h-3.5 text-emerald-400" />;
      case 'document':
        return <FileText className="w-3.5 h-3.5 text-blue-400" />;
      case 'code':
        return <Code className="w-3.5 h-3.5 text-amber-400" />;
      case 'archive':
        return <Archive className="w-3.5 h-3.5 text-orange-400" />;
      case 'model3d':
        return <Box className="w-3.5 h-3.5 text-rose-400" />;
      default:
        return <FileQuestion className="w-3.5 h-3.5 text-neutral-400" />;
    }
  };

  return (
    <div className="space-y-5 max-w-6xl">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold text-neutral-100 flex items-center space-x-2">
            <span>{t.collections.title}</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-neutral-800 text-neutral-400 font-normal">
              {collections.length}
            </span>
          </h3>
          <p className="text-xs text-neutral-400 mt-0.5">{t.collections.subtitle}</p>
        </div>

        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium shadow-sm transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{t.collections.newCollectionBtn}</span>
        </button>
      </div>

      {/* Collections Grid */}
      {collections.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <BookmarkCheck className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.collections.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.collections.emptyDesc}
          </p>
          <button
            onClick={() => setShowCreateModal(true)}
            className="inline-flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-emerald-600/90 hover:bg-emerald-600 text-white text-xs font-medium transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>{t.collections.newCollectionBtn}</span>
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3.5">
          {collections.map((col) => (
            <div
              key={col.id}
              onClick={() => setActiveCollection(col)}
              className="group cursor-pointer p-4 rounded-xl bg-neutral-900 border border-neutral-800 hover:border-neutral-700 hover:bg-neutral-850/80 transition-all flex flex-col justify-between"
            >
              <div>
                <div className="flex items-start justify-between gap-2 mb-2">
                  <div className="p-2 rounded-lg bg-neutral-950 border border-neutral-800 group-hover:border-emerald-500/30 transition-colors">
                    <FolderHeart className="w-4 h-4 text-emerald-400" />
                  </div>
                  <button
                    onClick={(e) => handleDeleteCollection(col.id, e)}
                    className="opacity-0 group-hover:opacity-100 p-1 text-neutral-500 hover:text-red-400 rounded hover:bg-neutral-800 transition-opacity"
                    title={t.collections.deleteCollection}
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>

                <h4 className="text-sm font-semibold text-neutral-100 group-hover:text-emerald-300 transition-colors mb-1">
                  {col.title}
                </h4>
                {col.description && (
                  <p className="text-xs text-neutral-400 line-clamp-2 leading-relaxed">
                    {col.description}
                  </p>
                )}
              </div>

              <div className="mt-4 pt-2.5 border-t border-neutral-800/60 flex items-center justify-between text-[10px] text-neutral-500">
                <span className="flex items-center space-x-1">
                  <Calendar className="w-3 h-3" />
                  <span>{new Date(col.updated_at).toLocaleDateString()}</span>
                </span>
                <span className="text-neutral-400 group-hover:text-emerald-400 transition-colors">
                  管理集合内容 →
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Create Collection Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-md w-full p-6 shadow-2xl space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-base font-semibold text-neutral-100 flex items-center space-x-2">
                <FolderHeart className="w-4 h-4 text-emerald-400" />
                <span>{t.collections.modalTitle}</span>
              </h4>
              <button
                onClick={() => setShowCreateModal(false)}
                className="text-neutral-500 hover:text-neutral-300"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form onSubmit={handleCreateCollection} className="space-y-3.5">
              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1">
                  {t.collections.titleLabel} *
                </label>
                <input
                  type="text"
                  required
                  placeholder={t.collections.titlePlaceholder}
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1">
                  {t.collections.descLabel}
                </label>
                <textarea
                  rows={2}
                  placeholder={t.collections.descPlaceholder}
                  value={newDesc}
                  onChange={(e) => setNewDesc(e.target.value)}
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 resize-none"
                />
              </div>

              <div className="flex items-center justify-end space-x-2 pt-2 border-t border-neutral-800/80">
                <button
                  type="button"
                  onClick={() => setShowCreateModal(false)}
                  className="px-4 py-2 text-xs rounded-lg text-neutral-400 hover:text-neutral-200 transition-colors"
                >
                  {t.common.cancel}
                </button>
                <button
                  type="submit"
                  disabled={creating}
                  className="px-4 py-2 text-xs font-medium rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white transition-colors disabled:opacity-50 shadow"
                >
                  {creating ? t.common.creating : t.common.create}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Collection Detail Modal */}
      {activeCollection && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-2xl w-full p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto">
            <div className="flex items-start justify-between">
              <div>
                <div className="flex items-center space-x-2 mb-1">
                  <FolderHeart className="w-4 h-4 text-emerald-400" />
                  <span className="text-[10px] text-neutral-500 font-mono">
                    ID: {activeCollection.id}
                  </span>
                </div>
                <h3 className="text-lg font-bold text-white">{activeCollection.title}</h3>
                {activeCollection.description && (
                  <p className="text-xs text-neutral-400 mt-1 leading-relaxed">
                    {activeCollection.description}
                  </p>
                )}
              </div>

              <div className="flex items-center space-x-1.5">
                <button
                  onClick={() => handleDeleteCollection(activeCollection.id)}
                  className="p-1.5 text-neutral-400 hover:text-red-400 rounded-lg bg-neutral-800 hover:bg-neutral-700 transition-colors"
                  title={t.collections.deleteCollection}
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={() => setActiveCollection(null)}
                  className="p-1.5 text-neutral-500 hover:text-neutral-300 rounded-lg hover:bg-neutral-800"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Collection Items Section */}
            <div className="space-y-3 pt-2 border-t border-neutral-800">
              <div className="flex items-center justify-between">
                <h5 className="text-xs font-semibold text-neutral-200 flex items-center space-x-1.5">
                  <BookmarkCheck className="w-3.5 h-3.5 text-emerald-400" />
                  <span>{t.collections.collectionDetail}</span>
                  <span className="text-[10px] px-1.5 py-0.2 rounded-full bg-neutral-800 text-neutral-400 font-normal">
                    {resolvedItems.length}
                  </span>
                </h5>

                <button
                  onClick={() => {
                    setItemSearchQuery('');
                    setShowAddItemModal(true);
                  }}
                  className="flex items-center space-x-1 text-xs px-2.5 py-1 rounded-lg bg-emerald-600/90 hover:bg-emerald-600 text-white font-medium transition-colors shadow-sm"
                >
                  <Plus className="w-3 h-3" />
                  <span>{t.collections.addItemBtn}</span>
                </button>
              </div>

              {loadingItems ? (
                <div className="py-8 text-center text-xs text-neutral-500">
                  加载集合内容中...
                </div>
              ) : resolvedItems.length === 0 ? (
                <div className="p-6 rounded-xl bg-neutral-950 border border-neutral-850 text-center space-y-2">
                  <p className="text-xs text-neutral-500">{t.collections.emptyItems}</p>
                </div>
              ) : (
                <div className="space-y-2 max-h-72 overflow-y-auto pr-1">
                  {resolvedItems.map(({ type, raw, data }) => {
                    if (type === 'asset') {
                      const asset = data as Asset | undefined;
                      const fileName = asset?.path?.split(/[\\/]/).pop() || raw.item_id;

                      return (
                        <div
                          key={raw.item_id}
                          className="p-2.5 rounded-lg bg-neutral-950 border border-neutral-800/80 hover:border-neutral-700 flex items-center justify-between transition-colors"
                        >
                          <div className="flex items-center space-x-2.5 min-w-0 pr-2">
                            {getKindIcon(asset?.kind || 'other')}
                            <div className="min-w-0">
                              <h6 className="text-xs font-medium text-neutral-200 truncate">
                                {fileName}
                              </h6>
                              <p className="text-[10px] text-neutral-500 font-mono truncate max-w-sm">
                                {asset?.path || raw.item_id}
                              </p>
                            </div>
                          </div>

                          <div className="flex items-center space-x-1">
                            <span className="text-[10px] text-neutral-500 font-mono mr-2">
                              {formatSize(asset?.size || null)}
                            </span>
                            {asset?.path && (
                              <button
                                onClick={() => handleReveal(asset.path)}
                                className="p-1 text-neutral-400 hover:text-neutral-100 rounded hover:bg-neutral-850"
                                title="在资源管理器中打开"
                              >
                                <ExternalLink className="w-3 h-3" />
                              </button>
                            )}
                            <button
                              onClick={() => handleRemoveItem(raw.item_id)}
                              className="p-1 text-neutral-400 hover:text-red-400 rounded hover:bg-neutral-850"
                              title={t.collections.removeItem}
                            >
                              <X className="w-3.5 h-3.5" />
                            </button>
                          </div>
                        </div>
                      );
                    } else {
                      // Entity item
                      const entity = data as Entity | undefined;

                      return (
                        <div
                          key={raw.item_id}
                          className="p-2.5 rounded-lg bg-neutral-950 border border-neutral-800/80 hover:border-neutral-700 flex items-center justify-between transition-colors"
                        >
                          <div className="flex items-center space-x-2.5 min-w-0 pr-2">
                            <div className="p-1 rounded bg-indigo-950 text-indigo-400">
                              <Boxes className="w-3.5 h-3.5" />
                            </div>
                            <div className="min-w-0">
                              <div className="flex items-center space-x-1.5">
                                <h6 className="text-xs font-medium text-neutral-200 truncate">
                                  {entity?.title || raw.item_id}
                                </h6>
                                <span className="text-[9px] px-1.5 py-0.2 rounded bg-indigo-500/10 text-indigo-400 font-mono">
                                  #{entity?.entity_type}
                                </span>
                              </div>
                              {entity?.description && (
                                <p className="text-[10px] text-neutral-500 truncate max-w-sm">
                                  {entity.description}
                                </p>
                              )}
                            </div>
                          </div>

                          <button
                            onClick={() => handleRemoveItem(raw.item_id)}
                            className="p-1 text-neutral-400 hover:text-red-400 rounded hover:bg-neutral-850"
                            title={t.collections.removeItem}
                          >
                            <X className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      );
                    }
                  })}
                </div>
              )}
            </div>

            <div className="flex justify-end pt-2">
              <button
                onClick={() => setActiveCollection(null)}
                className="px-4 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium transition-colors"
              >
                {t.assets.closeBtn}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Add Item to Collection Modal */}
      {showAddItemModal && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-lg w-full p-5 shadow-2xl space-y-4 max-h-[85vh] flex flex-col">
            <div className="flex items-center justify-between">
              <h4 className="text-sm font-semibold text-neutral-100 flex items-center space-x-2">
                <BookmarkCheck className="w-4 h-4 text-emerald-400" />
                <span>添加内容到集合</span>
              </h4>
              <button
                onClick={() => setShowAddItemModal(false)}
                className="text-neutral-500 hover:text-neutral-300"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Tabs: Assets vs Entities */}
            <div className="flex items-center space-x-1 bg-neutral-950 p-1 rounded-lg border border-neutral-800">
              <button
                onClick={() => setAddItemTab('asset')}
                className={`flex-1 py-1 text-xs rounded font-medium transition-colors ${
                  addItemTab === 'asset'
                    ? 'bg-neutral-800 text-white'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                数字资产 ({availableAssetsToAdd.length})
              </button>
              <button
                onClick={() => setAddItemTab('entity')}
                className={`flex-1 py-1 text-xs rounded font-medium transition-colors ${
                  addItemTab === 'entity'
                    ? 'bg-neutral-800 text-white'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                实体概念 ({availableEntitiesToAdd.length})
              </button>
            </div>

            {/* Search filter */}
            <div className="relative">
              <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
              <input
                type="text"
                value={itemSearchQuery}
                onChange={(e) => setItemSearchQuery(e.target.value)}
                placeholder="搜索名称或路径..."
                className="w-full pl-8 pr-3 py-1.5 text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-neutral-700 font-mono"
              />
            </div>

            {/* Available items list */}
            <div className="flex-1 overflow-y-auto space-y-1.5 max-h-64 pr-1">
              {addItemTab === 'asset' ? (
                availableAssetsToAdd.length === 0 ? (
                  <div className="py-6 text-center text-xs text-neutral-500">无可选资产</div>
                ) : (
                  availableAssetsToAdd.map((asset) => {
                    const fileName = asset.path?.split(/[\\/]/).pop() || asset.id;
                    return (
                      <div
                        key={asset.id}
                        onClick={() => handleAddItem(asset.id, 'asset')}
                        className="p-2 rounded-lg bg-neutral-950 hover:bg-neutral-850/80 border border-neutral-850 hover:border-neutral-700 flex items-center justify-between cursor-pointer transition-colors"
                      >
                        <div className="flex items-center space-x-2.5 min-w-0 pr-2">
                          {getKindIcon(asset.kind)}
                          <div className="min-w-0">
                            <h6 className="text-xs font-medium text-neutral-200 truncate">
                              {fileName}
                            </h6>
                            <p className="text-[10px] text-neutral-500 font-mono truncate max-w-sm">
                              {asset.path}
                            </p>
                          </div>
                        </div>
                        <button className="text-xs px-2.5 py-1 rounded bg-emerald-600/80 hover:bg-emerald-600 text-white font-medium whitespace-nowrap">
                          添加
                        </button>
                      </div>
                    );
                  })
                )
              ) : availableEntitiesToAdd.length === 0 ? (
                <div className="py-6 text-center text-xs text-neutral-500">无可选实体</div>
              ) : (
                availableEntitiesToAdd.map((entity) => (
                  <div
                    key={entity.id}
                    onClick={() => handleAddItem(entity.id, 'entity')}
                    className="p-2 rounded-lg bg-neutral-950 hover:bg-neutral-850/80 border border-neutral-850 hover:border-neutral-700 flex items-center justify-between cursor-pointer transition-colors"
                  >
                    <div className="flex items-center space-x-2.5 min-w-0 pr-2">
                      <Tag className="w-3.5 h-3.5 text-indigo-400" />
                      <div className="min-w-0">
                        <div className="flex items-center space-x-1.5">
                          <h6 className="text-xs font-medium text-neutral-200 truncate">
                            {entity.title}
                          </h6>
                          <span className="text-[9px] px-1.5 py-0.2 rounded bg-indigo-500/10 text-indigo-400 font-mono">
                            #{entity.entity_type}
                          </span>
                        </div>
                        {entity.description && (
                          <p className="text-[10px] text-neutral-500 truncate max-w-sm">
                            {entity.description}
                          </p>
                        )}
                      </div>
                    </div>
                    <button className="text-xs px-2.5 py-1 rounded bg-emerald-600/80 hover:bg-emerald-600 text-white font-medium whitespace-nowrap">
                      添加
                    </button>
                  </div>
                ))
              )}
            </div>

            <div className="flex justify-end pt-2 border-t border-neutral-800/80">
              <button
                onClick={() => setShowAddItemModal(false)}
                className="px-3 py-1.5 text-xs rounded-lg text-neutral-400 hover:text-neutral-200"
              >
                {t.common.cancel}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

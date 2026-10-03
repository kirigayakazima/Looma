import React, { useState, useMemo, useEffect, useCallback } from 'react';
import { Memory, Entity, Asset, Relation } from '../types';
import {
  BookOpen,
  Plus,
  Calendar,
  Pencil,
  Trash2,
  Search,
  Tag,
  X,
  Layers,
  FileText,
  Link2,
} from 'lucide-react';
import { useI18n } from '../i18n';
import { invoke } from '@tauri-apps/api/core';
import { MarkdownRenderer } from './MarkdownRenderer';

interface MemoriesViewProps {
  memories: Memory[];
  entities: Entity[];
  assets: Asset[];
  onRefresh: () => Promise<void>;
}

export const MemoriesView: React.FC<MemoriesViewProps> = ({
  memories,
  entities,
  assets,
  onRefresh,
}) => {
  const { t } = useI18n();

  // Create modal state
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [title, setTitle] = useState('');
  const [category, setCategory] = useState('journal');
  const [content, setContent] = useState('');
  const [selectedEntityId, setSelectedEntityId] = useState<string>('');
  const [loading, setLoading] = useState(false);

  // Edit modal state
  const [editingMemory, setEditingMemory] = useState<Memory | null>(null);
  const [editTitle, setEditTitle] = useState('');
  const [editCategory, setEditCategory] = useState('');
  const [editContent, setEditContent] = useState('');
  const [editLoading, setEditLoading] = useState(false);

  // Filter & Search state
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');

  // Relations map: memoryId -> Relation[]
  const [relationsMap, setRelationsMap] = useState<Record<string, Relation[]>>({});
  const [activeLinkDropdown, setActiveLinkDropdown] = useState<string | null>(null);

  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  // Load relations for all visible memories
  const loadAllRelations = useCallback(async () => {
    const map: Record<string, Relation[]> = {};
    for (const mem of memories) {
      const rels = await safeInvoke<Relation[]>('list_relations_for_item', { itemId: mem.id });
      if (rels) {
        map[mem.id] = rels;
      }
    }
    setRelationsMap(map);
  }, [memories]);

  useEffect(() => {
    loadAllRelations();
  }, [loadAllRelations]);

  // Distinct categories
  const categories = useMemo(() => {
    const set = new Set<string>();
    memories.forEach((m) => {
      if (m.category && m.category.trim()) {
        set.add(m.category.trim());
      }
    });
    return Array.from(set);
  }, [memories]);

  // Filtered memories
  const filteredMemories = useMemo(() => {
    return memories.filter((mem) => {
      if (selectedCategory !== 'all' && mem.category !== selectedCategory) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const tMatch = mem.title.toLowerCase().includes(q);
        const cMatch = mem.content.toLowerCase().includes(q);
        const catMatch = (mem.category || '').toLowerCase().includes(q);
        if (!tMatch && !cMatch && !catMatch) return false;
      }
      return true;
    });
  }, [memories, selectedCategory, searchQuery]);

  const handleCreateSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !content.trim()) return;
    setLoading(true);
    try {
      const memoryId = 'mem_' + Math.random().toString(36).substring(2, 9);
      const memory: Memory = {
        id: memoryId,
        title: title.trim(),
        content: content.trim(),
        category: category.trim() || null,
        metadata: {},
        recorded_at: new Date().toISOString(),
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      await safeInvoke('create_memory', { memory });

      // If an entity was selected to link at creation time
      if (selectedEntityId) {
        const rel: Relation = {
          id: 'rel_' + Math.random().toString(36).substring(2, 9),
          source_id: memoryId,
          source_type: 'memory',
          relation_type: 'refers_to',
          target_id: selectedEntityId,
          target_type: 'entity',
          metadata: {},
          created_at: new Date().toISOString(),
        };
        await safeInvoke('create_relation', { relation: rel });
      }

      setTitle('');
      setContent('');
      setSelectedEntityId('');
      setShowCreateModal(false);
      await onRefresh();
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const handleOpenEdit = (mem: Memory) => {
    setEditingMemory(mem);
    setEditTitle(mem.title);
    setEditCategory(mem.category || '');
    setEditContent(mem.content);
  };

  const handleEditSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingMemory || !editTitle.trim() || !editContent.trim()) return;
    setEditLoading(true);
    try {
      const updated: Memory = {
        ...editingMemory,
        title: editTitle.trim(),
        category: editCategory.trim() || null,
        content: editContent.trim(),
        updated_at: new Date().toISOString(),
      };
      await safeInvoke('update_memory', { memory: updated });
      setEditingMemory(null);
      await onRefresh();
    } catch (err) {
      console.error(err);
    } finally {
      setEditLoading(false);
    }
  };

  const handleDelete = async (id: string, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    if (!window.confirm(t.memories.deleteConfirm)) return;
    await safeInvoke('delete_memory', { id });
    await onRefresh();
  };

  const handleLinkItem = async (memoryId: string, targetId: string, targetType: string) => {
    const rel: Relation = {
      id: 'rel_' + Math.random().toString(36).substring(2, 9),
      source_id: memoryId,
      source_type: 'memory',
      relation_type: targetType === 'entity' ? 'refers_to' : 'attaches',
      target_id: targetId,
      target_type: targetType,
      metadata: {},
      created_at: new Date().toISOString(),
    };
    await safeInvoke('create_relation', { relation: rel });
    setActiveLinkDropdown(null);
    await loadAllRelations();
  };

  const handleUnlink = async (memoryId: string, targetId: string) => {
    await safeInvoke('delete_relation_between', { sourceId: memoryId, targetId });
    await loadAllRelations();
  };

  return (
    <div className="space-y-4 max-w-5xl">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <h3 className="text-base font-semibold text-neutral-100 flex items-center space-x-2">
            <span>{t.memories.title}</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-neutral-800 text-neutral-400 font-normal">
              {memories.length}
            </span>
          </h3>
          <p className="text-xs text-neutral-400">{t.memories.subtitle}</p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center space-x-1.5 text-xs px-3 py-1.5 rounded-md bg-amber-600 hover:bg-amber-500 text-white font-medium transition-colors shadow-sm shadow-amber-600/20"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{t.memories.writeBtn}</span>
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-2.5 bg-neutral-900/60 border border-neutral-800 p-2.5 rounded-xl">
        <div className="relative flex-1">
          <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
          <input
            type="text"
            placeholder={t.memories.searchPlaceholder}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-neutral-800/80 border border-neutral-700/60 rounded-lg pl-8 pr-3 py-1.5 text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-amber-500"
          />
        </div>

        {/* Category selector */}
        <div className="flex items-center space-x-1 overflow-x-auto text-xs py-0.5">
          <button
            onClick={() => setSelectedCategory('all')}
            className={`px-2.5 py-1 rounded-md text-xs whitespace-nowrap transition-colors ${
              selectedCategory === 'all'
                ? 'bg-amber-600/20 text-amber-300 border border-amber-500/30 font-medium'
                : 'bg-neutral-800 text-neutral-400 hover:text-neutral-200 border border-neutral-700/50'
            }`}
          >
            {t.memories.filterAllCategories}
          </button>
          {categories.map((cat) => (
            <button
              key={cat}
              onClick={() => setSelectedCategory(cat)}
              className={`px-2.5 py-1 rounded-md text-xs whitespace-nowrap transition-colors ${
                selectedCategory === cat
                  ? 'bg-amber-600/20 text-amber-300 border border-amber-500/30 font-medium'
                : 'bg-neutral-800 text-neutral-400 hover:text-neutral-200 border border-neutral-700/50'
              }`}
            >
              {cat}
            </button>
          ))}
        </div>
      </div>

      {/* Create Modal */}
      {showCreateModal && (
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-700/80 shadow-lg space-y-3">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-semibold text-neutral-200">{t.memories.modalTitle}</h4>
            <button
              onClick={() => setShowCreateModal(false)}
              className="text-neutral-400 hover:text-neutral-200"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
          <form onSubmit={handleCreateSubmit} className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <div>
                <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.memories.titleLabel}</label>
                <input
                  type="text"
                  required
                  placeholder={t.memories.titlePlaceholder}
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-amber-500"
                />
              </div>
              <div>
                <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.memories.categoryLabel}</label>
                <input
                  type="text"
                  placeholder={t.memories.categoryPlaceholder}
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                  className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-amber-500"
                />
              </div>
              <div>
                <label className="block text-[11px] font-medium text-neutral-400 mb-1">关联实体概念 (选填)</label>
                <select
                  value={selectedEntityId}
                  onChange={(e) => setSelectedEntityId(e.target.value)}
                  className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-amber-500"
                >
                  <option value="">-- 无关联 --</option>
                  {entities.map((ent) => (
                    <option key={ent.id} value={ent.id}>
                      {ent.title} ({ent.entity_type})
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <div>
              <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.memories.contentLabel} (支持标准 Markdown 语法)</label>
              <textarea
                rows={6}
                required
                placeholder={t.memories.contentPlaceholder}
                value={content}
                onChange={(e) => setContent(e.target.value)}
                className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-amber-500 font-mono"
              />
            </div>
            <div className="flex justify-end space-x-2">
              <button
                type="button"
                onClick={() => setShowCreateModal(false)}
                className="text-xs px-3 py-1.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
              >
                {t.common.cancel}
              </button>
              <button
                type="submit"
                disabled={loading}
                className="text-xs px-3 py-1.5 rounded bg-amber-600 hover:bg-amber-500 text-white font-medium transition-colors disabled:opacity-50"
              >
                {loading ? t.common.saving : t.common.save}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Edit Modal */}
      {editingMemory && (
        <div className="p-4 rounded-xl bg-neutral-900 border border-amber-600/50 shadow-lg space-y-3">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-semibold text-amber-400">{t.memories.editModalTitle}</h4>
            <button
              onClick={() => setEditingMemory(null)}
              className="text-neutral-400 hover:text-neutral-200"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
          <form onSubmit={handleEditSubmit} className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.memories.titleLabel}</label>
                <input
                  type="text"
                  required
                  value={editTitle}
                  onChange={(e) => setEditTitle(e.target.value)}
                  className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-amber-500"
                />
              </div>
              <div>
                <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.memories.categoryLabel}</label>
                <input
                  type="text"
                  value={editCategory}
                  onChange={(e) => setEditCategory(e.target.value)}
                  className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-amber-500"
                />
              </div>
            </div>
            <div>
              <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.memories.contentLabel} (Markdown)</label>
              <textarea
                rows={6}
                required
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-amber-500 font-mono"
              />
            </div>
            <div className="flex justify-end space-x-2">
              <button
                type="button"
                onClick={() => setEditingMemory(null)}
                className="text-xs px-3 py-1.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
              >
                {t.common.cancel}
              </button>
              <button
                type="submit"
                disabled={editLoading}
                className="text-xs px-3 py-1.5 rounded bg-amber-600 hover:bg-amber-500 text-white font-medium transition-colors disabled:opacity-50"
              >
                {editLoading ? t.common.saving : t.common.save}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Memories List */}
      {filteredMemories.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <BookOpen className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.memories.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.memories.emptyDesc}
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {filteredMemories.map((mem) => {
            const rels = relationsMap[mem.id] || [];
            const linkedEntityIds = rels
              .map((r) => (r.source_id === mem.id ? r.target_id : r.source_id));
            const linkedEntitiesList = entities.filter((e) => linkedEntityIds.includes(e.id));
            const linkedAssetsList = assets.filter((a) => linkedEntityIds.includes(a.id));

            return (
              <div
                key={mem.id}
                className="p-5 rounded-xl bg-neutral-900 border border-neutral-800 hover:border-neutral-700/80 transition-all group"
              >
                <div className="flex items-center justify-between mb-3 border-b border-neutral-800/60 pb-2.5">
                  <div className="flex items-center space-x-2.5">
                    <span className="text-sm font-semibold text-neutral-100">{mem.title}</span>
                    {mem.category && (
                      <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] bg-amber-500/10 text-amber-400 font-mono border border-amber-500/20">
                        <Tag className="w-2.5 h-2.5" />
                        <span>{mem.category}</span>
                      </span>
                    )}
                  </div>

                  <div className="flex items-center space-x-3">
                    <div className="flex items-center space-x-1 text-[11px] text-neutral-500">
                      <Calendar className="w-3 h-3" />
                      <span>{new Date(mem.recorded_at).toLocaleDateString()}</span>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center space-x-1 opacity-70 group-hover:opacity-100 transition-opacity">
                      <button
                        onClick={() => handleOpenEdit(mem)}
                        className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-amber-400 transition-colors"
                        title={t.memories.editBtn}
                      >
                        <Pencil className="w-3.5 h-3.5" />
                      </button>
                      <button
                        onClick={(e) => handleDelete(mem.id, e)}
                        className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-rose-400 transition-colors"
                        title={t.memories.deleteBtn}
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                </div>

                {/* Markdown Rendered Content */}
                <div className="py-1">
                  <MarkdownRenderer content={mem.content} />
                </div>

                {/* Linked Entities & Assets badges */}
                <div className="mt-4 pt-3 border-t border-neutral-800/60 flex flex-wrap items-center gap-2">
                  <span className="text-[11px] text-neutral-500 flex items-center space-x-1">
                    <Link2 className="w-3 h-3" />
                    <span>已关联概念与资产:</span>
                  </span>

                  {linkedEntitiesList.map((ent) => (
                    <span
                      key={ent.id}
                      className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-[11px] bg-indigo-500/10 text-indigo-300 border border-indigo-500/20"
                    >
                      <Layers className="w-2.5 h-2.5" />
                      <span>{ent.title}</span>
                      <button
                        onClick={() => handleUnlink(mem.id, ent.id)}
                        className="hover:text-rose-400 ml-0.5"
                        title="解除关联"
                      >
                        <X className="w-2.5 h-2.5" />
                      </button>
                    </span>
                  ))}

                  {linkedAssetsList.map((ast) => {
                    const fname = (ast.path || ast.id).split(/[\\/]/).pop() || ast.id;
                    return (
                      <span
                        key={ast.id}
                        className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-[11px] bg-sky-500/10 text-sky-300 border border-sky-500/20"
                      >
                        <FileText className="w-2.5 h-2.5" />
                        <span className="max-w-[140px] truncate">{fname}</span>
                        <button
                          onClick={() => handleUnlink(mem.id, ast.id)}
                          className="hover:text-rose-400 ml-0.5"
                          title="解除关联"
                        >
                          <X className="w-2.5 h-2.5" />
                        </button>
                      </span>
                    );
                  })}

                  {/* Add link dropdown toggle */}
                  <div className="relative">
                    <button
                      onClick={() =>
                        setActiveLinkDropdown(activeLinkDropdown === mem.id ? null : mem.id)
                      }
                      className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-[11px] bg-neutral-800 hover:bg-neutral-750 text-neutral-400 hover:text-neutral-200 border border-neutral-700/60 transition-colors"
                    >
                      <Plus className="w-2.5 h-2.5" />
                      <span>关联</span>
                    </button>

                    {activeLinkDropdown === mem.id && (
                      <div className="absolute left-0 top-full mt-1.5 z-30 w-56 bg-neutral-900 border border-neutral-700 rounded-lg shadow-xl p-2 space-y-2 text-xs">
                        <div>
                          <p className="text-[10px] font-semibold text-neutral-400 mb-1 px-1">关联实体概念</p>
                          <div className="max-h-32 overflow-y-auto space-y-1">
                            {entities.length === 0 ? (
                              <p className="text-[10px] text-neutral-500 px-1">暂无实体</p>
                            ) : (
                              entities
                                .filter((e) => !linkedEntityIds.includes(e.id))
                                .map((e) => (
                                  <button
                                    key={e.id}
                                    onClick={() => handleLinkItem(mem.id, e.id, 'entity')}
                                    className="w-full text-left px-2 py-1 rounded hover:bg-neutral-800 text-neutral-200 flex items-center justify-between text-xs"
                                  >
                                    <span className="truncate">{e.title}</span>
                                    <span className="text-[9px] text-neutral-500">{e.entity_type}</span>
                                  </button>
                                ))
                            )}
                          </div>
                        </div>

                        <div className="border-t border-neutral-800 pt-1.5">
                          <p className="text-[10px] font-semibold text-neutral-400 mb-1 px-1">关联数字资产</p>
                          <div className="max-h-32 overflow-y-auto space-y-1">
                            {assets.length === 0 ? (
                              <p className="text-[10px] text-neutral-500 px-1">暂无资产</p>
                            ) : (
                              assets
                                .filter((a) => !linkedEntityIds.includes(a.id))
                                .slice(0, 15)
                                .map((a) => {
                                  const name = (a.path || a.id).split(/[\\/]/).pop() || a.id;
                                  return (
                                    <button
                                      key={a.id}
                                      onClick={() => handleLinkItem(mem.id, a.id, 'asset')}
                                      className="w-full text-left px-2 py-1 rounded hover:bg-neutral-800 text-neutral-200 truncate text-xs"
                                    >
                                      {name}
                                    </button>
                                  );
                                })
                            )}
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

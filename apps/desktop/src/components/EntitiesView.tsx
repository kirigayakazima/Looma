import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Boxes,
  Plus,
  Tag,
  Search,
  X,
  Trash2,
  Edit2,
  ExternalLink,
  Unlink,
  Link as LinkIcon,
  FolderOpen,
  Image as ImageIcon,
  FileText,
  Video,
  Music,
  Archive,
  Code,
  Box,
  FileQuestion,
  Calendar,
  Sparkles,
} from 'lucide-react';
import { Entity, Asset, Relation } from '../types';
import { useI18n } from '../i18n';

interface EntitiesViewProps {
  entities: Entity[];
  assets: Asset[];
  onRefresh: () => Promise<void> | void;
}

export const EntitiesView: React.FC<EntitiesViewProps> = ({
  entities,
  assets,
  onRefresh,
}) => {
  const { t } = useI18n();

  // Search & Type filter
  const [selectedType, setSelectedType] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Create / Edit modal state
  const [showEditModal, setShowEditModal] = useState(false);
  const [editingEntityId, setEditingEntityId] = useState<string | null>(null);
  const [formTitle, setFormTitle] = useState('');
  const [formType, setFormType] = useState('project');
  const [formDesc, setFormDesc] = useState('');
  const [formProps, setFormProps] = useState<{ key: string; value: string }[]>([]);
  const [submitting, setSubmitting] = useState(false);

  // Detail modal state
  const [activeEntity, setActiveEntity] = useState<Entity | null>(null);
  const [entityRelations, setEntityRelations] = useState<Relation[]>([]);
  const [loadingRelations, setLoadingRelations] = useState(false);

  // Link asset modal state
  const [showLinkAssetModal, setShowLinkAssetModal] = useState(false);
  const [assetSearchQuery, setAssetSearchQuery] = useState('');

  // Safe invoke helper
  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  // Load relations for active entity
  const loadRelations = useCallback(async (entityId: string) => {
    setLoadingRelations(true);
    try {
      const rels = await safeInvoke<Relation[]>('list_relations_for_item', { itemId: entityId });
      setEntityRelations(rels || []);
    } finally {
      setLoadingRelations(false);
    }
  }, []);

  useEffect(() => {
    if (activeEntity) {
      loadRelations(activeEntity.id);
    } else {
      setEntityRelations([]);
    }
  }, [activeEntity, loadRelations]);

  // Available entity types for filtering
  const existingTypes = useMemo(() => {
    const types = new Set<string>();
    entities.forEach((e) => types.add(e.entity_type));
    return Array.from(types);
  }, [entities]);

  // Filtered entities list
  const filteredEntities = useMemo(() => {
    return entities.filter((e) => {
      if (selectedType !== 'all' && e.entity_type !== selectedType) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matchesTitle = e.title.toLowerCase().includes(q);
        const matchesDesc = (e.description || '').toLowerCase().includes(q);
        const matchesType = e.entity_type.toLowerCase().includes(q);
        const matchesProps = JSON.stringify(e.properties).toLowerCase().includes(q);
        if (!matchesTitle && !matchesDesc && !matchesType && !matchesProps) {
          return false;
        }
      }
      return true;
    });
  }, [entities, selectedType, searchQuery]);

  // Open modal to create new entity
  const handleOpenCreate = () => {
    setEditingEntityId(null);
    setFormTitle('');
    setFormType('anime');
    setFormDesc('');
    setFormProps([]);
    setShowEditModal(true);
  };

  // Open modal to edit existing entity
  const handleOpenEdit = (entity: Entity, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setEditingEntityId(entity.id);
    setFormTitle(entity.title);
    setFormType(entity.entity_type);
    setFormDesc(entity.description || '');

    const propsList = Object.entries(entity.properties || {}).map(([k, v]) => ({
      key: k,
      value: typeof v === 'object' ? JSON.stringify(v) : String(v),
    }));
    setFormProps(propsList);
    setShowEditModal(true);
  };

  // Add a dynamic property row
  const handleAddPropRow = () => {
    setFormProps([...formProps, { key: '', value: '' }]);
  };

  // Remove a dynamic property row
  const handleRemovePropRow = (index: number) => {
    setFormProps(formProps.filter((_, i) => i !== index));
  };

  // Update dynamic property row
  const handleUpdatePropRow = (index: number, field: 'key' | 'value', val: string) => {
    const updated = [...formProps];
    updated[index][field] = val;
    setFormProps(updated);
  };

  // Save entity (create or update)
  const handleSaveEntity = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formTitle.trim()) return;
    setSubmitting(true);

    try {
      const propertiesObj: Record<string, unknown> = {};
      formProps.forEach(({ key, value }) => {
        const trimmedKey = key.trim();
        if (trimmedKey) {
          propertiesObj[trimmedKey] = value.trim();
        }
      });

      if (editingEntityId) {
        // Update existing entity
        const existing = entities.find((ent) => ent.id === editingEntityId);
        const updatedEntity: Entity = {
          id: editingEntityId,
          title: formTitle.trim(),
          entity_type: formType.trim(),
          description: formDesc.trim() || null,
          properties: propertiesObj,
          created_at: existing?.created_at || new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };

        await safeInvoke('update_entity', { entity: updatedEntity });
        if (activeEntity?.id === editingEntityId) {
          setActiveEntity(updatedEntity);
        }
      } else {
        // Create new entity
        const newEntity: Entity = {
          id: 'ent_' + Math.random().toString(36).substring(2, 9),
          title: formTitle.trim(),
          entity_type: formType.trim(),
          description: formDesc.trim() || null,
          properties: propertiesObj,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };

        await safeInvoke('create_entity', { entity: newEntity });
      }

      setShowEditModal(false);
      await onRefresh();
    } finally {
      setSubmitting(false);
    }
  };

  // Delete entity
  const handleDeleteEntity = async (id: string, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    if (!window.confirm(t.entities.deleteConfirm)) return;
    await safeInvoke('delete_entity', { id });
    if (activeEntity?.id === id) {
      setActiveEntity(null);
    }
    await onRefresh();
  };

  // Link an asset to active entity
  const handleLinkAsset = async (assetId: string) => {
    if (!activeEntity) return;

    const relation: Relation = {
      id: 'rel_' + Math.random().toString(36).substring(2, 9),
      source_id: assetId,
      source_type: 'asset',
      relation_type: 'belongs_to',
      target_id: activeEntity.id,
      target_type: 'entity',
      metadata: {},
      created_at: new Date().toISOString(),
    };

    await safeInvoke('create_relation', { relation });
    await loadRelations(activeEntity.id);
    setShowLinkAssetModal(false);
  };

  // Unlink an asset from active entity
  const handleUnlinkAsset = async (assetId: string) => {
    if (!activeEntity) return;
    await safeInvoke('delete_relation_between', {
      sourceId: assetId,
      targetId: activeEntity.id,
    });
    await loadRelations(activeEntity.id);
  };

  // Get assets currently linked to active entity
  const linkedAssets = useMemo(() => {
    if (!activeEntity) return [];
    const assetIdSet = new Set(
      entityRelations
        .filter((r) => r.source_type === 'asset' || r.target_type === 'asset')
        .map((r) => (r.source_type === 'asset' ? r.source_id : r.target_id))
    );
    return assets.filter((a) => assetIdSet.has(a.id));
  }, [activeEntity, entityRelations, assets]);

  // Assets available to link (exclude already linked)
  const availableAssetsToLink = useMemo(() => {
    const linkedIds = new Set(linkedAssets.map((a) => a.id));
    return assets
      .filter((a) => !linkedIds.has(a.id))
      .filter((a) => {
        if (!assetSearchQuery.trim()) return true;
        const q = assetSearchQuery.toLowerCase();
        return (a.path || '').toLowerCase().includes(q) || a.id.includes(q);
      });
  }, [assets, linkedAssets, assetSearchQuery]);

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

  const presetTypes = ['anime', 'project', 'device', 'game', 'book', 'person', 'place'];

  return (
    <div className="space-y-5 max-w-6xl">
      {/* Header Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold text-neutral-100 flex items-center space-x-2">
            <span>{t.entities.title}</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-neutral-800 text-neutral-400 font-normal">
              {entities.length}
            </span>
          </h3>
          <p className="text-xs text-neutral-400 mt-0.5">{t.entities.subtitle}</p>
        </div>

        <button
          onClick={handleOpenCreate}
          className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium shadow-sm transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{t.entities.newEntityBtn}</span>
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-3 bg-neutral-900/60 border border-neutral-800/80 p-3 rounded-xl">
        {/* Type Filter Chips */}
        <div className="flex items-center space-x-1 overflow-x-auto pb-1 md:pb-0 scrollbar-none">
          <button
            onClick={() => setSelectedType('all')}
            className={`text-xs px-2.5 py-1 rounded-md transition-colors whitespace-nowrap ${
              selectedType === 'all'
                ? 'bg-neutral-800 text-neutral-100 font-medium'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850'
            }`}
          >
            {t.entities.filterAll}
          </button>
          {existingTypes.map((type) => (
            <button
              key={type}
              onClick={() => setSelectedType(type)}
              className={`text-xs px-2.5 py-1 rounded-md transition-colors whitespace-nowrap ${
                selectedType === type
                  ? 'bg-indigo-600/30 text-indigo-300 font-medium border border-indigo-500/40'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850'
              }`}
            >
              #{type}
            </button>
          ))}
        </div>

        {/* Search Input */}
        <div className="relative min-w-[240px]">
          <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.entities.searchPlaceholder}
            className="w-full pl-8 pr-3 py-1 text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-neutral-700"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery('')}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-500 hover:text-neutral-300"
            >
              <X className="w-3 h-3" />
            </button>
          )}
        </div>
      </div>

      {/* Entities Grid */}
      {filteredEntities.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <Boxes className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.entities.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.entities.emptyDesc}
          </p>
          <button
            onClick={handleOpenCreate}
            className="inline-flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-indigo-600/90 hover:bg-indigo-600 text-white text-xs font-medium transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>{t.entities.newEntityBtn}</span>
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3.5">
          {filteredEntities.map((item) => {
            const propsEntries = Object.entries(item.properties || {});

            return (
              <div
                key={item.id}
                onClick={() => setActiveEntity(item)}
                className="group cursor-pointer p-4 rounded-xl bg-neutral-900 border border-neutral-800 hover:border-neutral-700 hover:bg-neutral-850/80 transition-all flex flex-col justify-between"
              >
                <div>
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] bg-indigo-500/10 text-indigo-400 font-mono border border-indigo-500/20">
                      <Tag className="w-2.5 h-2.5 mr-0.5" />
                      {item.entity_type}
                    </span>
                    <div className="flex items-center space-x-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        onClick={(e) => handleOpenEdit(item, e)}
                        className="p-1 text-neutral-400 hover:text-neutral-100 rounded hover:bg-neutral-800"
                        title={t.entities.editEntity}
                      >
                        <Edit2 className="w-3 h-3" />
                      </button>
                      <button
                        onClick={(e) => handleDeleteEntity(item.id, e)}
                        className="p-1 text-neutral-400 hover:text-red-400 rounded hover:bg-neutral-800"
                        title={t.entities.deleteEntity}
                      >
                        <Trash2 className="w-3 h-3" />
                      </button>
                    </div>
                  </div>

                  <h4 className="text-sm font-semibold text-neutral-100 group-hover:text-white transition-colors mb-1">
                    {item.title}
                  </h4>
                  {item.description && (
                    <p className="text-xs text-neutral-400 line-clamp-2 mb-2 leading-relaxed">
                      {item.description}
                    </p>
                  )}

                  {/* Properties Pills */}
                  {propsEntries.length > 0 && (
                    <div className="flex flex-wrap gap-1.5 mt-2.5">
                      {propsEntries.slice(0, 3).map(([k, v]) => (
                        <span
                          key={k}
                          className="text-[10px] px-2 py-0.5 rounded bg-neutral-950 border border-neutral-800/80 text-neutral-300 font-mono"
                        >
                          <span className="text-neutral-500">{k}:</span> {String(v)}
                        </span>
                      ))}
                      {propsEntries.length > 3 && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-neutral-950 text-neutral-500 font-mono">
                          +{propsEntries.length - 3}
                        </span>
                      )}
                    </div>
                  )}
                </div>

                <div className="mt-4 pt-2.5 border-t border-neutral-800/60 flex items-center justify-between text-[10px] text-neutral-500">
                  <span className="flex items-center space-x-1">
                    <Calendar className="w-3 h-3" />
                    <span>{new Date(item.updated_at).toLocaleDateString()}</span>
                  </span>
                  <span className="text-neutral-400 group-hover:text-indigo-400 transition-colors">
                    查看详情 & 关联资产 →
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Create / Edit Entity Modal */}
      {showEditModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-lg w-full p-6 shadow-2xl space-y-4 max-h-[90vh] overflow-y-auto">
            <div className="flex items-center justify-between">
              <h4 className="text-base font-semibold text-neutral-100 flex items-center space-x-2">
                <Sparkles className="w-4 h-4 text-indigo-400" />
                <span>
                  {editingEntityId ? t.entities.editModalTitle : t.entities.modalTitle}
                </span>
              </h4>
              <button
                onClick={() => setShowEditModal(false)}
                className="text-neutral-500 hover:text-neutral-300"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form onSubmit={handleSaveEntity} className="space-y-4">
              {/* Title & Type */}
              <div className="space-y-3">
                <div>
                  <label className="block text-xs font-medium text-neutral-300 mb-1">
                    {t.entities.titleLabel} *
                  </label>
                  <input
                    type="text"
                    required
                    placeholder={t.entities.titlePlaceholder}
                    value={formTitle}
                    onChange={(e) => setFormTitle(e.target.value)}
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-neutral-300 mb-1">
                    {t.entities.typeLabel} *
                  </label>
                  <div className="flex items-center space-x-2 mb-1.5">
                    <input
                      type="text"
                      required
                      placeholder={t.entities.typePlaceholder}
                      value={formType}
                      onChange={(e) => setFormType(e.target.value)}
                      className="flex-1 bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500 font-mono"
                    />
                  </div>
                  {/* Quick preset types */}
                  <div className="flex flex-wrap gap-1">
                    {presetTypes.map((pt) => (
                      <button
                        type="button"
                        key={pt}
                        onClick={() => setFormType(pt)}
                        className={`text-[10px] px-2 py-0.5 rounded font-mono transition-colors ${
                          formType === pt
                            ? 'bg-indigo-600/30 text-indigo-300 border border-indigo-500/40'
                            : 'bg-neutral-950 text-neutral-500 hover:text-neutral-300 border border-neutral-850'
                        }`}
                      >
                        {pt}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              {/* Description */}
              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1">
                  {t.entities.descLabel}
                </label>
                <textarea
                  rows={2}
                  placeholder={t.entities.descPlaceholder}
                  value={formDesc}
                  onChange={(e) => setFormDesc(e.target.value)}
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500 resize-none"
                />
              </div>

              {/* Dynamic Key-Value Properties */}
              <div className="space-y-2 pt-2 border-t border-neutral-800/80">
                <div className="flex items-center justify-between">
                  <label className="text-xs font-medium text-neutral-300">
                    {t.entities.propertiesLabel}
                  </label>
                  <button
                    type="button"
                    onClick={handleAddPropRow}
                    className="flex items-center space-x-1 text-[11px] text-indigo-400 hover:text-indigo-300 transition-colors"
                  >
                    <Plus className="w-3 h-3" />
                    <span>{t.entities.addProperty}</span>
                  </button>
                </div>

                {formProps.length === 0 ? (
                  <p className="text-[11px] text-neutral-500 italic">
                    暂未添加自定义属性。支持为动漫添加评分、为项目添加语言与仓库、为硬件添加规格等。
                  </p>
                ) : (
                  <div className="space-y-2 max-h-48 overflow-y-auto pr-1">
                    {formProps.map((p, idx) => (
                      <div key={idx} className="flex items-center space-x-2">
                        <input
                          type="text"
                          placeholder={t.entities.propKey}
                          value={p.key}
                          onChange={(e) => handleUpdatePropRow(idx, 'key', e.target.value)}
                          className="w-1/3 bg-neutral-950 border border-neutral-800 rounded px-2.5 py-1 text-xs text-neutral-200 font-mono focus:outline-none focus:border-neutral-700"
                        />
                        <input
                          type="text"
                          placeholder={t.entities.propValue}
                          value={p.value}
                          onChange={(e) => handleUpdatePropRow(idx, 'value', e.target.value)}
                          className="flex-1 bg-neutral-950 border border-neutral-800 rounded px-2.5 py-1 text-xs text-neutral-200 font-mono focus:outline-none focus:border-neutral-700"
                        />
                        <button
                          type="button"
                          onClick={() => handleRemovePropRow(idx)}
                          className="p-1 text-neutral-500 hover:text-red-400 rounded transition-colors"
                        >
                          <X className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Actions */}
              <div className="flex items-center justify-end space-x-2 pt-3 border-t border-neutral-800/80">
                <button
                  type="button"
                  onClick={() => setShowEditModal(false)}
                  className="px-4 py-2 text-xs rounded-lg text-neutral-400 hover:text-neutral-200 transition-colors"
                >
                  {t.common.cancel}
                </button>
                <button
                  type="submit"
                  disabled={submitting}
                  className="px-4 py-2 text-xs font-medium rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors disabled:opacity-50 shadow"
                >
                  {submitting ? t.common.saving : t.common.save}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Entity Detail & Linked Assets Drawer / Modal */}
      {activeEntity && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-2xl w-full p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto">
            {/* Modal Header */}
            <div className="flex items-start justify-between">
              <div>
                <div className="flex items-center space-x-2 mb-1.5">
                  <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] bg-indigo-500/10 text-indigo-400 font-mono border border-indigo-500/20">
                    <Tag className="w-2.5 h-2.5 mr-0.5" />
                    {activeEntity.entity_type}
                  </span>
                  <span className="text-[10px] text-neutral-500">
                    {t.entities.createdTime}: {new Date(activeEntity.created_at).toLocaleDateString()}
                  </span>
                </div>
                <h3 className="text-lg font-bold text-white">{activeEntity.title}</h3>
                {activeEntity.description && (
                  <p className="text-xs text-neutral-300 mt-1 leading-relaxed">
                    {activeEntity.description}
                  </p>
                )}
              </div>

              <div className="flex items-center space-x-1.5">
                <button
                  onClick={() => handleOpenEdit(activeEntity)}
                  className="p-1.5 text-neutral-400 hover:text-neutral-200 rounded-lg bg-neutral-800 hover:bg-neutral-700 transition-colors"
                  title={t.entities.editEntity}
                >
                  <Edit2 className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={() => handleDeleteEntity(activeEntity.id)}
                  className="p-1.5 text-neutral-400 hover:text-red-400 rounded-lg bg-neutral-800 hover:bg-neutral-700 transition-colors"
                  title={t.entities.deleteEntity}
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={() => setActiveEntity(null)}
                  className="p-1.5 text-neutral-500 hover:text-neutral-300 rounded-lg hover:bg-neutral-800"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Custom Properties Grid */}
            {Object.keys(activeEntity.properties || {}).length > 0 && (
              <div className="p-3.5 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-2">
                <h5 className="text-[11px] font-semibold text-neutral-400 uppercase tracking-wider">
                  {t.entities.propertiesLabel}
                </h5>
                <div className="grid grid-cols-2 sm:grid-cols-3 gap-2.5 text-xs">
                  {Object.entries(activeEntity.properties).map(([k, v]) => (
                    <div key={k} className="bg-neutral-900/80 p-2 rounded-lg border border-neutral-850">
                      <span className="text-neutral-500 text-[10px] block font-mono uppercase">{k}</span>
                      <span className="text-neutral-200 font-medium font-mono text-[11px] break-all">
                        {String(v)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Linked Assets Section */}
            <div className="space-y-3 pt-2 border-t border-neutral-800">
              <div className="flex items-center justify-between">
                <div>
                  <h5 className="text-xs font-semibold text-neutral-200 flex items-center space-x-1.5">
                    <LinkIcon className="w-3.5 h-3.5 text-emerald-400" />
                    <span>{t.entities.linkedAssets}</span>
                    <span className="text-[10px] px-1.5 py-0.2 rounded-full bg-neutral-800 text-neutral-400 font-normal">
                      {linkedAssets.length}
                    </span>
                  </h5>
                </div>

                <button
                  onClick={() => {
                    setAssetSearchQuery('');
                    setShowLinkAssetModal(true);
                  }}
                  className="flex items-center space-x-1 text-xs px-2.5 py-1 rounded-lg bg-emerald-600/90 hover:bg-emerald-600 text-white font-medium transition-colors shadow-sm"
                >
                  <Plus className="w-3 h-3" />
                  <span>{t.entities.linkAssetBtn}</span>
                </button>
              </div>

              {loadingRelations ? (
                <div className="py-6 text-center text-xs text-neutral-500">
                  加载关联资产中...
                </div>
              ) : linkedAssets.length === 0 ? (
                <div className="p-6 rounded-xl bg-neutral-950 border border-neutral-850 text-center space-y-2">
                  <p className="text-xs text-neutral-500">{t.entities.noLinkedAssets}</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 max-h-60 overflow-y-auto pr-1">
                  {linkedAssets.map((asset) => {
                    const fileName = asset.path?.split(/[\\/]/).pop() || asset.id;

                    return (
                      <div
                        key={asset.id}
                        className="p-2.5 rounded-lg bg-neutral-950 border border-neutral-800/80 hover:border-neutral-700 flex items-center justify-between transition-colors"
                      >
                        <div className="flex items-center space-x-2.5 min-w-0 pr-2">
                          {getKindIcon(asset.kind)}
                          <div className="min-w-0">
                            <h6 className="text-xs font-medium text-neutral-200 truncate" title={fileName}>
                              {fileName}
                            </h6>
                            <span className="text-[10px] text-neutral-500 font-mono">
                              {formatSize(asset.size)}
                            </span>
                          </div>
                        </div>

                        <div className="flex items-center space-x-1">
                          <button
                            onClick={() => {
                              if (asset.path) {
                                safeInvoke('open_in_file_manager', { path: asset.path });
                              }
                            }}
                            className="p-1 text-neutral-400 hover:text-neutral-100 rounded hover:bg-neutral-850"
                            title="在资源管理器中打开"
                          >
                            <ExternalLink className="w-3 h-3" />
                          </button>
                          <button
                            onClick={() => handleUnlinkAsset(asset.id)}
                            className="p-1 text-neutral-400 hover:text-red-400 rounded hover:bg-neutral-850"
                            title={t.entities.unlinkAsset}
                          >
                            <Unlink className="w-3 h-3" />
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Footer */}
            <div className="flex justify-end pt-2">
              <button
                onClick={() => setActiveEntity(null)}
                className="px-4 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium transition-colors"
              >
                {t.assets.closeBtn}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Select Asset to Link Modal */}
      {showLinkAssetModal && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-lg w-full p-5 shadow-2xl space-y-4 max-h-[85vh] flex flex-col">
            <div className="flex items-center justify-between">
              <h4 className="text-sm font-semibold text-neutral-100 flex items-center space-x-2">
                <FolderOpen className="w-4 h-4 text-emerald-400" />
                <span>{t.entities.linkAssetModalTitle}</span>
              </h4>
              <button
                onClick={() => setShowLinkAssetModal(false)}
                className="text-neutral-500 hover:text-neutral-300"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Search filter for assets */}
            <div className="relative">
              <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
              <input
                type="text"
                value={assetSearchQuery}
                onChange={(e) => setAssetSearchQuery(e.target.value)}
                placeholder={t.entities.searchAssetToLink}
                className="w-full pl-8 pr-3 py-1.5 text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-neutral-700 font-mono"
              />
            </div>

            {/* List of available assets */}
            <div className="flex-1 overflow-y-auto space-y-1.5 max-h-72 pr-1">
              {availableAssetsToLink.length === 0 ? (
                <div className="py-8 text-center text-xs text-neutral-500">
                  {assets.length === 0
                    ? '资产库暂无已索引的资产，请先在「数字资产」中扫描文件夹'
                    : '无匹配资产或所有资产已关联'}
                </div>
              ) : (
                availableAssetsToLink.map((asset) => {
                  const fileName = asset.path?.split(/[\\/]/).pop() || asset.id;

                  return (
                    <div
                      key={asset.id}
                      onClick={() => handleLinkAsset(asset.id)}
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

                      <button
                        className="text-xs px-2.5 py-1 rounded bg-indigo-600/80 hover:bg-indigo-600 text-white font-medium whitespace-nowrap transition-colors"
                      >
                        关联
                      </button>
                    </div>
                  );
                })
              )}
            </div>

            <div className="flex justify-end pt-2 border-t border-neutral-800/80">
              <button
                onClick={() => setShowLinkAssetModal(false)}
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

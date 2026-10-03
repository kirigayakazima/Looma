import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Film,
  Plus,
  Search,
  ExternalLink,
  ChevronRight,
  Tv,
  BookOpen,
  Gamepad2,
  Clapperboard,
  Music,
  Bookmark,
  CheckCircle2,
  Clock,
  PauseCircle,
  XCircle,
  Link as LinkIcon,
  Trash2,
  X,
  Image as ImageIcon,
  Minus,
} from 'lucide-react';
import { Entity, Asset, ExternalReference, WorkSummary, WorkType, RecordStatus } from '../types';
import { MarkdownRenderer } from './MarkdownRenderer';

interface WorksViewProps {
  assets: Asset[];
  onRefreshAll: () => Promise<void> | void;
}

export const WorksView: React.FC<WorksViewProps> = ({
  assets,
  onRefreshAll,
}) => {
  // List of works (entities representing works)
  const [works, setWorks] = useState<Entity[]>([]);
  const [loading, setLoading] = useState(false);

  // Filters
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Active work detail drawer
  const [activeWorkId, setActiveWorkId] = useState<string | null>(null);
  const [activeSummary, setActiveSummary] = useState<WorkSummary | null>(null);
  const [loadingSummary, setLoadingSummary] = useState(false);

  // Modals
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showAddRefModal, setShowAddRefModal] = useState(false);
  const [showLinkAssetModal, setShowLinkAssetModal] = useState(false);

  // Create Form State
  const [newTitle, setNewTitle] = useState('');
  const [newOrigTitle, setNewOrigTitle] = useState('');
  const [newType, setNewType] = useState<WorkType>('anime');
  const [newStatus, setNewStatus] = useState<RecordStatus>('in_progress');
  const [newDesc, setNewDesc] = useState('');
  const [newTotalPos, setNewTotalPos] = useState<string>('');
  const [creating, setCreating] = useState(false);

  // Add Ref Form State
  const [refProvider, setRefProvider] = useState('bilibili');
  const [refTitle, setRefTitle] = useState('');
  const [refUrl, setRefUrl] = useState('');
  const [addingRef, setAddingRef] = useState(false);

  // Link Asset Search
  const [assetSearch, setAssetSearch] = useState('');

  // Safe IPC invoke
  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  // Load works list
  const loadWorks = useCallback(async () => {
    setLoading(true);
    try {
      const list = await safeInvoke<Entity[]>('list_works', {
        workType: typeFilter === 'all' ? null : typeFilter,
        status: statusFilter === 'all' ? null : statusFilter,
      });
      setWorks(list || []);
    } finally {
      setLoading(false);
    }
  }, [typeFilter, statusFilter]);

  useEffect(() => {
    loadWorks();
  }, [loadWorks]);

  // Load active work summary
  const loadWorkSummary = useCallback(async (id: string) => {
    setLoadingSummary(true);
    try {
      const summary = await safeInvoke<WorkSummary>('get_work_summary', { id });
      setActiveSummary(summary || null);
    } finally {
      setLoadingSummary(false);
    }
  }, []);

  useEffect(() => {
    if (activeWorkId) {
      loadWorkSummary(activeWorkId);
    } else {
      setActiveSummary(null);
    }
  }, [activeWorkId, loadWorkSummary]);

  // Open external URL in system browser
  const handleOpenUrl = async (url: string) => {
    if (!url) return;
    try {
      await invoke('open_external_url', { url });
    } catch (e) {
      console.error('Failed to open URL:', e);
      window.open(url, '_blank');
    }
  };

  // Quick Stepper (+1 / -1)
  const handleStepProgress = async (work: Entity, delta: number, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    const meta = work.properties?.work as any;
    const current = meta?.progress?.position ?? 0;
    const next = Math.max(0, current + delta);
    const total = meta?.progress?.total_positions ?? null;
    const pType = meta?.progress?.position_type ?? null;
    const unit = meta?.progress?.unit ?? null;

    await safeInvoke('update_work_progress', {
      id: work.id,
      position: next,
      positionType: pType,
      totalPositions: total,
      unit,
    });

    await loadWorks();
    if (activeWorkId === work.id) {
      await loadWorkSummary(work.id);
    }
  };

  // Change Status
  const handleChangeStatus = async (id: string, newSt: RecordStatus) => {
    await safeInvoke('update_work_status', { id, status: newSt });
    await loadWorks();
    if (activeWorkId === id) {
      await loadWorkSummary(id);
    }
  };

  // Create Work
  const handleCreateWork = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;
    setCreating(true);
    try {
      const totalNum = newTotalPos ? parseFloat(newTotalPos) : null;
      const created = await safeInvoke<Entity>('create_work', {
        title: newTitle.trim(),
        kind: newType,
        status: newStatus,
        originalTitle: newOrigTitle.trim() || null,
        description: newDesc.trim() || null,
      });

      if (created && totalNum) {
        // Set initial progress total
        await safeInvoke('update_work_progress', {
          id: created.id,
          position: newStatus === 'completed' ? totalNum : 0,
          positionType: null,
          totalPositions: totalNum,
          unit: null,
        });
      }

      setShowCreateModal(false);
      setNewTitle('');
      setNewOrigTitle('');
      setNewDesc('');
      setNewTotalPos('');
      await loadWorks();
      await onRefreshAll();
    } finally {
      setCreating(false);
    }
  };

  // Add External Reference
  const handleAddRef = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeWorkId || !refUrl.trim() || !refTitle.trim()) return;
    setAddingRef(true);
    try {
      const newRef: ExternalReference = {
        id: 'ref_' + Math.random().toString(36).substring(2, 9),
        entity_id: activeWorkId,
        provider: refProvider,
        title: refTitle.trim(),
        url: refUrl.trim(),
        description: null,
        metadata: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      await safeInvoke('create_external_reference', { reference: newRef });
      setShowAddRefModal(false);
      setRefTitle('');
      setRefUrl('');
      await loadWorkSummary(activeWorkId);
    } finally {
      setAddingRef(false);
    }
  };

  // Delete External Reference
  const handleDeleteRef = async (refId: string) => {
    if (!activeWorkId) return;
    await safeInvoke('delete_external_reference', { id: refId });
    await loadWorkSummary(activeWorkId);
  };

  // Link Asset to Work
  const handleLinkAsset = async (assetId: string) => {
    if (!activeWorkId) return;
    const relation = {
      id: 'rel_' + Math.random().toString(36).substring(2, 9),
      source_id: activeWorkId,
      source_type: 'entity',
      relation_type: 'attaches',
      target_id: assetId,
      target_type: 'asset',
      metadata: {},
      created_at: new Date().toISOString(),
    };
    await safeInvoke('create_relation', { relation });
    setShowLinkAssetModal(false);
    await loadWorkSummary(activeWorkId);
  };

  // Unlink Asset
  const handleUnlinkAsset = async (assetId: string) => {
    if (!activeWorkId) return;
    await safeInvoke('delete_relation_between', {
      sourceId: activeWorkId,
      targetId: assetId,
    });
    await safeInvoke('delete_relation_between', {
      sourceId: assetId,
      targetId: activeWorkId,
    });
    await loadWorkSummary(activeWorkId);
  };

  // Filtered Works List
  const filteredWorks = useMemo(() => {
    return works.filter((w) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      const meta = w.properties?.work as any;
      const titleMatch = w.title.toLowerCase().includes(q);
      const origMatch = (meta?.original_title || '').toLowerCase().includes(q);
      const descMatch = (w.description || '').toLowerCase().includes(q);
      return titleMatch || origMatch || descMatch;
    });
  }, [works, searchQuery]);

  // Icons by type
  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'anime':
        return <Tv className="w-4 h-4 text-purple-400" />;
      case 'manga':
        return <BookOpen className="w-4 h-4 text-amber-400" />;
      case 'game':
        return <Gamepad2 className="w-4 h-4 text-emerald-400" />;
      case 'movie':
      case 'tv_series':
        return <Clapperboard className="w-4 h-4 text-rose-400" />;
      case 'music':
      case 'album':
        return <Music className="w-4 h-4 text-sky-400" />;
      default:
        return <Film className="w-4 h-4 text-indigo-400" />;
    }
  };

  // Type labels
  const getTypeLabel = (type: string) => {
    switch (type) {
      case 'anime': return '动画';
      case 'manga': return '漫画';
      case 'game': return '游戏';
      case 'movie': return '电影';
      case 'tv_series': return '剧集';
      case 'book':
      case 'novel': return '书籍';
      case 'music': return '音乐';
      default: return '作品';
    }
  };

  // Status styles & labels
  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'in_progress':
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            <Clock className="w-3 h-3" />
            <span>进行中</span>
          </span>
        );
      case 'completed':
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-xs font-medium bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">
            <CheckCircle2 className="w-3 h-3" />
            <span>已完成</span>
          </span>
        );
      case 'planned':
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
            <Bookmark className="w-3 h-3" />
            <span>计划中</span>
          </span>
        );
      case 'paused':
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-xs font-medium bg-neutral-700 text-neutral-300">
            <PauseCircle className="w-3 h-3" />
            <span>搁置中</span>
          </span>
        );
      case 'dropped':
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-xs font-medium bg-rose-500/10 text-rose-400">
            <XCircle className="w-3 h-3" />
            <span>已放弃</span>
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-xs font-medium bg-neutral-800 text-neutral-400">
            <span>{status}</span>
          </span>
        );
    }
  };

  return (
    <div className="space-y-5 max-w-6xl pb-12">
      {/* Top Banner & Action */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-bold text-neutral-100 flex items-center space-x-2">
            <span>作品档案与进度追踪</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 font-normal">
              {filteredWorks.length} 部作品
            </span>
          </h2>
          <p className="text-xs text-neutral-400 mt-1">
            记录您消费番剧、漫画、游戏与书籍的个人足迹，关联本地切片与在线主页
          </p>
        </div>

        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center space-x-1.5 px-3.5 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium shadow-sm shadow-indigo-600/20 transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>新建作品档案</span>
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="space-y-2 bg-neutral-900/60 border border-neutral-800/80 p-3 rounded-xl">
        {/* Type Filter Chips */}
        <div className="flex items-center space-x-1 overflow-x-auto pb-1 scrollbar-none">
          {[
            { id: 'all', label: '全部类别' },
            { id: 'anime', label: '动画 / 番剧' },
            { id: 'manga', label: '漫画' },
            { id: 'game', label: '游戏' },
            { id: 'movie', label: '电影 / 剧集' },
            { id: 'book', label: '书籍 / 小说' },
            { id: 'music', label: '音乐' },
          ].map((item) => (
            <button
              key={item.id}
              onClick={() => setTypeFilter(item.id)}
              className={`text-xs px-3 py-1 rounded-md transition-colors whitespace-nowrap ${
                typeFilter === item.id
                  ? 'bg-neutral-800 text-neutral-100 font-medium'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850'
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>

        {/* Status Filter & Search */}
        <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 pt-2 border-t border-neutral-800/50">
          <div className="flex items-center space-x-1 overflow-x-auto scrollbar-none">
            {[
              { id: 'all', label: '全部状态' },
              { id: 'in_progress', label: '进行中' },
              { id: 'planned', label: '计划中' },
              { id: 'completed', label: '已完成' },
              { id: 'paused', label: '搁置' },
            ].map((item) => (
              <button
                key={item.id}
                onClick={() => setStatusFilter(item.id)}
                className={`text-xs px-2.5 py-0.5 rounded-full transition-colors whitespace-nowrap ${
                  statusFilter === item.id
                    ? 'bg-indigo-600/20 text-indigo-300 font-medium border border-indigo-500/30'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                {item.label}
              </button>
            ))}
          </div>

          <div className="relative w-full sm:w-64">
            <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="搜索作品名称、原名或介绍..."
              className="w-full bg-neutral-950 border border-neutral-800 rounded-lg pl-8 pr-3 py-1 text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-indigo-500 transition-colors"
            />
          </div>
        </div>
      </div>

      {/* Grid of Works */}
      {loading ? (
        <div className="flex items-center justify-center py-16 text-xs text-neutral-500">
          加载作品档案中...
        </div>
      ) : filteredWorks.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 bg-neutral-900/40 border border-dashed border-neutral-800 rounded-2xl">
          <Film className="w-10 h-10 text-neutral-600 mb-3" />
          <p className="text-sm text-neutral-300 font-medium">暂无匹配的作品档案</p>
          <p className="text-xs text-neutral-500 mt-1">
            点击上方 “新建作品档案” 开始登记您的动画、游戏或漫画追更记录
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredWorks.map((work) => {
            const meta = work.properties?.work as any;
            const progress = meta?.progress;
            const pos = progress?.position ?? 0;
            const total = progress?.total_positions ?? null;
            const unit = progress?.unit || (work.entity_type === 'manga' ? '话' : '集');
            const percent = total && total > 0 ? Math.min(100, Math.round((pos / total) * 100)) : null;

            return (
              <div
                key={work.id}
                onClick={() => setActiveWorkId(work.id)}
                className="group relative bg-neutral-900/70 hover:bg-neutral-850/90 border border-neutral-800/80 hover:border-neutral-700/80 rounded-xl p-4 transition-all duration-200 cursor-pointer flex flex-col justify-between"
              >
                <div>
                  {/* Top Badges */}
                  <div className="flex items-center justify-between gap-2 mb-2">
                    <div className="flex items-center space-x-1.5">
                      {getTypeIcon(work.entity_type)}
                      <span className="text-xs font-medium text-neutral-300">
                        {getTypeLabel(work.entity_type)}
                      </span>
                    </div>
                    {getStatusBadge(meta?.status || 'planned')}
                  </div>

                  {/* Title & Native Title */}
                  <h4 className="text-sm font-semibold text-neutral-100 group-hover:text-indigo-400 transition-colors line-clamp-1">
                    {work.title}
                  </h4>
                  {meta?.original_title && (
                    <p className="text-xs text-neutral-500 line-clamp-1 mt-0.5">
                      {meta.original_title}
                    </p>
                  )}

                  {work.description && (
                    <p className="text-xs text-neutral-400 line-clamp-2 mt-2 leading-relaxed">
                      {work.description}
                    </p>
                  )}
                </div>

                {/* Progress Stepper & Bar */}
                <div className="mt-4 pt-3 border-t border-neutral-800/60">
                  <div className="flex items-center justify-between text-xs mb-1.5">
                    <span className="text-neutral-400">进度:</span>
                    <span className="font-mono text-neutral-200 font-medium">
                      第 {pos} {unit}
                      {total ? ` / ${total} ${unit}` : ''}
                      {percent !== null ? ` (${percent}%)` : ''}
                    </span>
                  </div>

                  {/* Progress Bar */}
                  {percent !== null && (
                    <div className="w-full bg-neutral-800 rounded-full h-1.5 overflow-hidden mb-2.5">
                      <div
                        className="bg-indigo-500 h-full rounded-full transition-all duration-300"
                        style={{ width: `${percent}%` }}
                      />
                    </div>
                  )}

                  {/* Quick Stepper Buttons */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-1.5">
                      <button
                        onClick={(e) => handleStepProgress(work, -1, e)}
                        className="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white transition-colors"
                        title="后退 1 进度"
                      >
                        <Minus className="w-3 h-3" />
                      </button>
                      <button
                        onClick={(e) => handleStepProgress(work, 1, e)}
                        className="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white transition-colors"
                        title="前进 1 进度"
                      >
                        <Plus className="w-3 h-3" />
                      </button>
                    </div>

                    <span className="text-[11px] text-neutral-500 group-hover:text-neutral-400 flex items-center">
                      <span>查看详情</span>
                      <ChevronRight className="w-3.5 h-3.5 ml-0.5" />
                    </span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Work Detail Modal / Drawer */}
      {activeWorkId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fadeIn">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-3xl max-h-[90vh] flex flex-col shadow-2xl overflow-hidden">
            {/* Modal Header */}
            <div className="p-5 border-b border-neutral-800 flex items-center justify-between">
              <div className="flex items-center space-x-3">
                <div className="p-2.5 rounded-xl bg-neutral-800 text-indigo-400">
                  {activeSummary ? getTypeIcon(activeSummary.entity.entity_type) : <Film className="w-5 h-5" />}
                </div>
                <div>
                  <h3 className="text-base font-bold text-neutral-100">
                    {activeSummary?.entity.title || '加载作品详情...'}
                  </h3>
                  {activeSummary?.work_metadata?.original_title && (
                    <p className="text-xs text-neutral-400">
                      {activeSummary.work_metadata.original_title}
                    </p>
                  )}
                </div>
              </div>

              <div className="flex items-center space-x-2">
                <button
                  onClick={() => setActiveWorkId(null)}
                  className="p-1.5 rounded-lg text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>
            </div>

            {/* Modal Body */}
            <div className="flex-1 overflow-y-auto p-5 space-y-6">
              {loadingSummary || !activeSummary ? (
                <div className="py-12 text-center text-xs text-neutral-500">
                  加载中...
                </div>
              ) : (
                <>
                  {/* Status & Progress Card */}
                  <div className="bg-neutral-950/80 border border-neutral-800/80 rounded-xl p-4 space-y-3">
                    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                      <div>
                        <span className="text-xs text-neutral-400">消费状态:</span>
                        <div className="flex items-center space-x-1.5 mt-1">
                          {(['in_progress', 'planned', 'completed', 'paused', 'dropped'] as RecordStatus[]).map((st) => (
                            <button
                              key={st}
                              onClick={() => handleChangeStatus(activeSummary.entity.id, st)}
                              className={`text-xs px-2.5 py-1 rounded-md transition-colors ${
                                activeSummary.work_metadata?.status === st
                                  ? 'bg-indigo-600 text-white font-medium'
                                  : 'bg-neutral-850 text-neutral-400 hover:text-neutral-200'
                              }`}
                            >
                              {st === 'in_progress' ? '进行中' : st === 'planned' ? '想看/玩' : st === 'completed' ? '已完成' : st === 'paused' ? '搁置' : '放弃'}
                            </button>
                          ))}
                        </div>
                      </div>

                      {/* Progress Controls */}
                      <div className="flex items-center space-x-2">
                        <div className="text-right">
                          <span className="text-xs text-neutral-400">当前进度:</span>
                          <p className="text-sm font-bold font-mono text-neutral-100">
                            第 {activeSummary.work_metadata?.progress?.position ?? 0}{' '}
                            {activeSummary.work_metadata?.progress?.unit || '集'}
                          </p>
                        </div>
                        <div className="flex items-center space-x-1">
                          <button
                            onClick={() => handleStepProgress(activeSummary.entity, -1)}
                            className="p-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 transition-colors"
                          >
                            <Minus className="w-3.5 h-3.5" />
                          </button>
                          <button
                            onClick={() => handleStepProgress(activeSummary.entity, 1)}
                            className="p-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 transition-colors"
                          >
                            <Plus className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* External References (网上入口 / Bilibili / Bangumi / Steam) */}
                  <div className="space-y-2.5">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-semibold text-neutral-300 uppercase tracking-wider flex items-center space-x-1.5">
                        <LinkIcon className="w-3.5 h-3.5 text-indigo-400" />
                        <span>外部在线入口 ({activeSummary.external_references.length})</span>
                      </h4>
                      <button
                        onClick={() => setShowAddRefModal(true)}
                        className="text-xs px-2.5 py-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-200 flex items-center space-x-1 transition-colors"
                      >
                        <Plus className="w-3 h-3" />
                        <span>添加入口</span>
                      </button>
                    </div>

                    {activeSummary.external_references.length === 0 ? (
                      <p className="text-xs text-neutral-500 py-3 text-center border border-dashed border-neutral-800 rounded-lg">
                        暂无外部在线入口，可添加 Bilibili、Bangumi、Steam 或官网页面
                      </p>
                    ) : (
                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                        {activeSummary.external_references.map((ref) => (
                          <div
                            key={ref.id}
                            className="flex items-center justify-between p-2.5 rounded-lg bg-neutral-950/60 border border-neutral-800 hover:border-neutral-700 transition-colors"
                          >
                            <div
                              onClick={() => handleOpenUrl(ref.url)}
                              className="flex items-center space-x-2 flex-1 cursor-pointer truncate"
                            >
                              <ExternalLink className="w-3.5 h-3.5 text-indigo-400 shrink-0" />
                              <div className="truncate">
                                <p className="text-xs font-medium text-neutral-200 truncate">
                                  {ref.title}
                                </p>
                                <p className="text-[11px] text-neutral-500 truncate">{ref.url}</p>
                              </div>
                            </div>
                            <button
                              onClick={() => handleDeleteRef(ref.id)}
                              className="p-1 text-neutral-500 hover:text-rose-400 transition-colors"
                            >
                              <Trash2 className="w-3 h-3" />
                            </button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Linked Assets (本地关联资源) */}
                  <div className="space-y-2.5">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-semibold text-neutral-300 uppercase tracking-wider flex items-center space-x-1.5">
                        <ImageIcon className="w-3.5 h-3.5 text-emerald-400" />
                        <span>关联本地文件 / 海报 ({activeSummary.linked_assets.length})</span>
                      </h4>
                      <button
                        onClick={() => setShowLinkAssetModal(true)}
                        className="text-xs px-2.5 py-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-200 flex items-center space-x-1 transition-colors"
                      >
                        <Plus className="w-3 h-3" />
                        <span>关联本地资产</span>
                      </button>
                    </div>

                    {activeSummary.linked_assets.length === 0 ? (
                      <p className="text-xs text-neutral-500 py-3 text-center border border-dashed border-neutral-800 rounded-lg">
                        暂未关联本地文件，点击上方按钮关联已扫描的壁纸或海报
                      </p>
                    ) : (
                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
                        {activeSummary.linked_assets.map((asset) => (
                          <div
                            key={asset.id}
                            className="relative group bg-neutral-950 border border-neutral-800 rounded-lg p-2 flex flex-col justify-between"
                          >
                            <div className="flex items-center space-x-1.5 text-xs text-neutral-300 truncate mb-1">
                              <ImageIcon className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                              <span className="truncate">{asset.path?.split(/[\\/]/).pop()}</span>
                            </div>
                            <div className="flex items-center justify-between pt-1 border-t border-neutral-850 text-[10px] text-neutral-500">
                              <span>{asset.kind}</span>
                              <button
                                onClick={() => handleUnlinkAsset(asset.id)}
                                className="text-rose-400 hover:underline"
                              >
                                解除
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Linked Memories (手记与感悟) */}
                  <div className="space-y-2.5">
                    <h4 className="text-xs font-semibold text-neutral-300 uppercase tracking-wider flex items-center space-x-1.5">
                      <BookOpen className="w-3.5 h-3.5 text-amber-400" />
                      <span>观后心得与日记 ({activeSummary.linked_memories.length})</span>
                    </h4>

                    {activeSummary.linked_memories.length === 0 ? (
                      <p className="text-xs text-neutral-500 py-3 text-center border border-dashed border-neutral-800 rounded-lg">
                        暂无心得手记，前往左侧「记忆手记」即可为该作品书写评测
                      </p>
                    ) : (
                      <div className="space-y-2">
                        {activeSummary.linked_memories.map((mem) => (
                          <div
                            key={mem.id}
                            className="p-3 rounded-lg bg-neutral-950/60 border border-neutral-800"
                          >
                            <div className="flex items-center justify-between mb-1.5">
                              <span className="text-xs font-semibold text-neutral-200">
                                {mem.title}
                              </span>
                              <span className="text-[11px] text-neutral-500">
                                {new Date(mem.recorded_at).toLocaleDateString()}
                              </span>
                            </div>
                            <div className="text-xs text-neutral-400">
                              <MarkdownRenderer content={mem.content} />
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Description */}
                  {activeSummary.entity.description && (
                    <div className="space-y-1.5 pt-2 border-t border-neutral-800">
                      <span className="text-xs text-neutral-400">剧情简介 / 备注:</span>
                      <p className="text-xs text-neutral-300 leading-relaxed bg-neutral-950/40 p-3 rounded-lg border border-neutral-850">
                        {activeSummary.entity.description}
                      </p>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Create Work Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fadeIn">
          <form
            onSubmit={handleCreateWork}
            className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-lg p-5 shadow-2xl space-y-4"
          >
            <div className="flex items-center justify-between pb-3 border-b border-neutral-800">
              <h3 className="text-base font-bold text-neutral-100 flex items-center space-x-2">
                <Plus className="w-4 h-4 text-indigo-400" />
                <span>登记新作品档案</span>
              </h3>
              <button
                type="button"
                onClick={() => setShowCreateModal(false)}
                className="text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="space-y-3">
              <div>
                <label className="text-xs font-medium text-neutral-300 block mb-1">
                  作品中文标题 <span className="text-rose-400">*</span>
                </label>
                <input
                  type="text"
                  required
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  placeholder="例如：BLEACH 千年血战篇 / 鸣潮 / 葬送的芙莉莲"
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs font-medium text-neutral-300 block mb-1">
                    原名 / 外文名
                  </label>
                  <input
                    type="text"
                    value={newOrigTitle}
                    onChange={(e) => setNewOrigTitle(e.target.value)}
                    placeholder="例如：BLEACH 千年血戦篇"
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
                  />
                </div>

                <div>
                  <label className="text-xs font-medium text-neutral-300 block mb-1">
                    作品类型
                  </label>
                  <select
                    value={newType}
                    onChange={(e) => setNewType(e.target.value as WorkType)}
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500"
                  >
                    <option value="anime">动画 / 番剧</option>
                    <option value="manga">漫画</option>
                    <option value="game">游戏</option>
                    <option value="movie">电影</option>
                    <option value="tv_series">连续剧</option>
                    <option value="book">书籍 / 小说</option>
                    <option value="music">音乐 / 专辑</option>
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs font-medium text-neutral-300 block mb-1">
                    当前状态
                  </label>
                  <select
                    value={newStatus}
                    onChange={(e) => setNewStatus(e.target.value as RecordStatus)}
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500"
                  >
                    <option value="in_progress">进行中 (在看/在玩)</option>
                    <option value="planned">计划中 (想看/想玩)</option>
                    <option value="completed">已完成 (通关/看完)</option>
                    <option value="paused">搁置中</option>
                  </select>
                </div>

                <div>
                  <label className="text-xs font-medium text-neutral-300 block mb-1">
                    总集数 / 总话数 (可选)
                  </label>
                  <input
                    type="number"
                    value={newTotalPos}
                    onChange={(e) => setNewTotalPos(e.target.value)}
                    placeholder="例如：24"
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
                  />
                </div>
              </div>

              <div>
                <label className="text-xs font-medium text-neutral-300 block mb-1">
                  作品简介 / 备注
                </label>
                <textarea
                  rows={3}
                  value={newDesc}
                  onChange={(e) => setNewDesc(e.target.value)}
                  placeholder="剧情简介、购买平台或初见感想..."
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
                />
              </div>
            </div>

            <div className="flex items-center justify-end space-x-2 pt-3 border-t border-neutral-800">
              <button
                type="button"
                onClick={() => setShowCreateModal(false)}
                className="px-3 py-1.5 rounded-lg text-xs text-neutral-400 hover:text-neutral-200"
              >
                取消
              </button>
              <button
                type="submit"
                disabled={creating}
                className="px-4 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium transition-colors disabled:opacity-50"
              >
                {creating ? '登记中...' : '确认登记'}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Add External Reference Modal */}
      {showAddRefModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fadeIn">
          <form
            onSubmit={handleAddRef}
            className="bg-neutral-900 border border-neutral-800 rounded-xl w-full max-w-md p-5 shadow-2xl space-y-4"
          >
            <div className="flex items-center justify-between pb-3 border-b border-neutral-800">
              <h4 className="text-sm font-bold text-neutral-100">添加外部在线入口</h4>
              <button
                type="button"
                onClick={() => setShowAddRefModal(false)}
                className="text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-3">
              <div>
                <label className="text-xs text-neutral-300 block mb-1">平台 / 来源</label>
                <select
                  value={refProvider}
                  onChange={(e) => setRefProvider(e.target.value)}
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100"
                >
                  <option value="bilibili">哔哩哔哩 (Bilibili)</option>
                  <option value="bangumi">Bangumi 番组计划</option>
                  <option value="steam">Steam 游戏主页</option>
                  <option value="website">官方网站</option>
                  <option value="github">GitHub</option>
                  <option value="other">其他链接</option>
                </select>
              </div>

              <div>
                <label className="text-xs text-neutral-300 block mb-1">显示名称</label>
                <input
                  type="text"
                  required
                  value={refTitle}
                  onChange={(e) => setRefTitle(e.target.value)}
                  placeholder="例如：B站追番入口 / Bangumi 条目"
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100"
                />
              </div>

              <div>
                <label className="text-xs text-neutral-300 block mb-1">完整网址 (URL)</label>
                <input
                  type="url"
                  required
                  value={refUrl}
                  onChange={(e) => setRefUrl(e.target.value)}
                  placeholder="https://..."
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 font-mono"
                />
              </div>
            </div>

            <div className="flex items-center justify-end space-x-2 pt-2 border-t border-neutral-800">
              <button
                type="button"
                onClick={() => setShowAddRefModal(false)}
                className="px-3 py-1 rounded text-xs text-neutral-400 hover:text-neutral-200"
              >
                取消
              </button>
              <button
                type="submit"
                disabled={addingRef}
                className="px-3.5 py-1 rounded bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium"
              >
                {addingRef ? '添加中...' : '确认添加'}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Link Asset Modal */}
      {showLinkAssetModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fadeIn">
          <div className="bg-neutral-900 border border-neutral-800 rounded-xl w-full max-w-lg p-5 shadow-2xl space-y-4 max-h-[80vh] flex flex-col">
            <div className="flex items-center justify-between pb-3 border-b border-neutral-800">
              <h4 className="text-sm font-bold text-neutral-100">选择要关联的本地文件</h4>
              <button
                onClick={() => setShowLinkAssetModal(false)}
                className="text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="relative">
              <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
              <input
                type="text"
                value={assetSearch}
                onChange={(e) => setAssetSearch(e.target.value)}
                placeholder="按文件名搜索已扫描的文件..."
                className="w-full bg-neutral-950 border border-neutral-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-neutral-100"
              />
            </div>

            <div className="flex-1 overflow-y-auto space-y-1.5 max-h-64">
              {assets
                .filter((a) => {
                  if (!assetSearch.trim()) return true;
                  const q = assetSearch.toLowerCase();
                  return (a.path || '').toLowerCase().includes(q);
                })
                .slice(0, 30)
                .map((a) => (
                  <div
                    key={a.id}
                    onClick={() => handleLinkAsset(a.id)}
                    className="flex items-center justify-between p-2 rounded-lg bg-neutral-950/60 hover:bg-neutral-800 border border-neutral-850 cursor-pointer transition-colors"
                  >
                    <div className="flex items-center space-x-2 truncate">
                      <ImageIcon className="w-4 h-4 text-emerald-400 shrink-0" />
                      <span className="text-xs text-neutral-200 truncate">
                        {a.path?.split(/[\\/]/).pop()}
                      </span>
                    </div>
                    <span className="text-[10px] text-indigo-400 font-medium shrink-0">点击关联</span>
                  </div>
                ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Film,
  Plus,
  Search,
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
  X,
  Minus,
} from 'lucide-react';
import { Entity, Asset, WorkType, RecordStatus } from '../types';
import { WorkDetailDrawer } from './WorkDetailDrawer';

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

  // Modals
  const [showCreateModal, setShowCreateModal] = useState(false);

  // Create Form State
  const [newTitle, setNewTitle] = useState('');
  const [newOrigTitle, setNewOrigTitle] = useState('');
  const [newAliases, setNewAliases] = useState('');
  const [newType, setNewType] = useState<WorkType>('anime');
  const [newStatus, setNewStatus] = useState<RecordStatus>('in_progress');
  const [newDesc, setNewDesc] = useState('');
  const [newTotalPos, setNewTotalPos] = useState<string>('');
  const [creating, setCreating] = useState(false);

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
  };

  // Create Work
  const handleCreateWork = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;
    setCreating(true);
    try {
      const totalNum = newTotalPos ? parseFloat(newTotalPos) : null;
      const aliasList = newAliases.split(/[,，]/).map(s => s.trim()).filter(Boolean);
      const created = await safeInvoke<Entity>('create_work', {
        title: newTitle.trim(),
        kind: newType,
        status: newStatus,
        originalTitle: newOrigTitle.trim() || null,
        description: newDesc.trim() || null,
        aliases: aliasList.length > 0 ? aliasList : null,
      });

      if (created) {
        // Set initial progress (works for both finite and ongoing works)
        await safeInvoke('update_work_progress', {
          id: created.id,
          position: newStatus === 'completed' && totalNum ? totalNum : 0,
          positionType: null,
          totalPositions: totalNum,
          unit: null,
        });
      }

      setShowCreateModal(false);
      setNewTitle('');
      setNewOrigTitle('');
      setNewAliases('');
      setNewDesc('');
      setNewTotalPos('');
      await loadWorks();
      await onRefreshAll();
    } finally {
      setCreating(false);
    }
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
      const aliasMatch = Array.isArray(meta?.aliases) && meta.aliases.some((a: string) => a.toLowerCase().includes(q));
      return titleMatch || origMatch || descMatch || aliasMatch;
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
      case 'archived':
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-xs font-medium bg-neutral-800 text-neutral-400 border border-neutral-700/50">
            <span>已封存</span>
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
              { id: 'archived', label: '已封存' },
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
                  {meta?.aliases && meta.aliases.length > 0 && (
                    <div className="flex flex-wrap gap-1 mt-1">
                      {meta.aliases.slice(0, 2).map((alias: string, i: number) => (
                        <span key={i} className="text-[10px] px-1.5 py-0.2 rounded bg-neutral-800 text-neutral-400 border border-neutral-700/50">
                          {alias}
                        </span>
                      ))}
                      {meta.aliases.length > 2 && (
                        <span className="text-[10px] text-neutral-500">
                          +{meta.aliases.length - 2}
                        </span>
                      )}
                    </div>
                  )}

                  {work.description && (
                    <p className="text-xs text-neutral-400 line-clamp-2 mt-2 leading-relaxed">
                      {work.description}
                    </p>
                  )}
                  {/* Type-specific metadata tags */}
                  {meta?.type_metadata && (
                    <div className="flex flex-wrap gap-1 mt-2">
                      {meta.type_metadata.studio && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-purple-500/10 text-purple-300 border border-purple-500/20">
                          {meta.type_metadata.studio}
                        </span>
                      )}
                      {meta.type_metadata.season && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-purple-500/10 text-purple-300 border border-purple-500/20">
                          {meta.type_metadata.season}
                        </span>
                      )}
                      {meta.type_metadata.author && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-300 border border-amber-500/20">
                          作者: {meta.type_metadata.author}
                        </span>
                      )}
                      {meta.type_metadata.director && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-rose-500/10 text-rose-300 border border-rose-500/20">
                          导演: {meta.type_metadata.director}
                        </span>
                      )}
                      {meta.type_metadata.isbn && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-sky-500/10 text-sky-300 border border-sky-500/20 font-mono">
                          ISBN: {meta.type_metadata.isbn}
                        </span>
                      )}
                    </div>
                  )}
                </div>

                {/* Progress Stepper & Bar */}
                <div className="mt-4 pt-3 border-t border-neutral-800/60">
                  <div className="flex items-center justify-between text-xs mb-1.5">
                    <span className="text-neutral-400">进度:</span>
                    <span className="font-mono text-neutral-200 font-medium">
                      第 {pos} {unit}
                      {total ? ` / ${total} ${unit}` : ' (连载/进行中)'}
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

      {/* Unified Work Detail Drawer */}
      {activeWorkId && (
        <WorkDetailDrawer
          workId={activeWorkId}
          onClose={() => setActiveWorkId(null)}
          onRefreshAll={async () => {
            await loadWorks();
            if (onRefreshAll) await onRefreshAll();
          }}
          availableAssets={assets}
        />
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
                    <option value="archived">已封存 / 归档</option>
                  </select>
                </div>

                <div>
                  <label className="text-xs font-medium text-neutral-300 block mb-1">
                    总集/话/卷/页数 (留空表示连载中)
                  </label>
                  <input
                    type="number"
                    value={newTotalPos}
                    onChange={(e) => setNewTotalPos(e.target.value)}
                    placeholder="留空表示连载中或未知上限"
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
                  />
                </div>
              </div>

              <div>
                <label className="text-xs font-medium text-neutral-300 block mb-1">
                  别名 / 译名 / 简称 (逗号分隔)
                </label>
                <input
                  type="text"
                  value={newAliases}
                  onChange={(e) => setNewAliases(e.target.value)}
                  placeholder="例如：死神, 死神千年血战, Bleach TYBW"
                  className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
                />
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

    </div>
  );
};

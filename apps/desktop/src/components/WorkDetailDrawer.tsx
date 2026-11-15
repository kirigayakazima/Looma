import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  X,
  ExternalLink,
  Plus,
  Compass,
  Archive,
  HardDrive,
  FolderOpen,
  Folder,
  Layers,
  Minus,
  Clock,
  Trash2,
  Gamepad2,
  AlertTriangle,
} from 'lucide-react';
import {
  Asset,
  WorkSummary,
  RecordStatus,
  ExternalReference,
  GameActivityRecord,
} from '../types';
import { ExperienceModal } from './ExperienceModal';

interface WorkDetailDrawerProps {
  workId: string | null;
  onClose: () => void;
  onRefreshAll?: () => Promise<void> | void;
  availableAssets: Asset[];
}

export const WorkDetailDrawer: React.FC<WorkDetailDrawerProps> = ({
  workId,
  onClose,
  onRefreshAll,
  availableAssets,
}) => {
  const [summary, setSummary] = useState<WorkSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Experience modal state
  const [showExpModal, setShowExpModal] = useState(false);
  const [expDefaultType, setExpDefaultType] = useState('completed');
  const [expDefaultPlatform, setExpDefaultPlatform] = useState('');
  const [expDefaultNote, setExpDefaultNote] = useState('');

  // Game activity records
  const [activities, setActivities] = useState<GameActivityRecord[]>([]);

  // Add Ref Modal State
  const [showAddRefModal, setShowAddRefModal] = useState(false);
  const [refProvider, setRefProvider] = useState('steam');
  const [refTitle, setRefTitle] = useState('');
  const [refUrl, setRefUrl] = useState('');
  const [addingRef, setAddingRef] = useState(false);

  // Link Asset Modal State
  const [showLinkAssetModal, setShowLinkAssetModal] = useState(false);
  const [assetSearchQuery, setAssetSearchQuery] = useState('');

  // Alias addition
  const [showAddAlias, setShowAddAlias] = useState(false);
  const [newAliasInput, setNewAliasInput] = useState('');

  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err: any) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      setErrorMsg(err?.message || String(err));
      return null;
    }
  };

  const loadSummary = useCallback(async () => {
    if (!workId) {
      setSummary(null);
      setActivities([]);
      return;
    }
    setLoading(true);
    setErrorMsg(null);
    try {
      const [data, acts] = await Promise.all([
        safeInvoke<WorkSummary>('get_work_summary', { id: workId }),
        safeInvoke<GameActivityRecord[]>('list_game_activities', { workId }),
      ]);
      if (data) setSummary(data);
      if (acts) setActivities(acts);
    } finally {
      setLoading(false);
    }
  }, [workId]);

  useEffect(() => {
    loadSummary();
  }, [loadSummary]);

  if (!workId) return null;

  const work = summary?.entity;
  const meta = summary?.work_metadata || (work?.properties?.work as any);
  const progress = meta?.progress;
  const aliases: string[] = meta?.aliases || [];

  // Status update
  const handleUpdateStatus = async (newStatus: RecordStatus) => {
    if (newStatus === 'completed') {
      setExpDefaultType('completed');
      setShowExpModal(true);
      return;
    }
    await safeInvoke('update_work_status', { id: workId, status: newStatus });
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  // Progress update
  const handleUpdateProgress = async (newPos: number) => {
    if (!workId) return;
    const pos = Math.max(0, newPos);
    await safeInvoke('update_work_progress', {
      entityId: workId,
      position: pos,
      positionType: progress?.position_type || 'episode',
      totalPositions: progress?.total_positions || null,
      unit: progress?.unit || null,
    });
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  // Add alias
  const handleAddAlias = async () => {
    if (!newAliasInput.trim() || !workId) return;
    const updated = Array.from(new Set([...aliases, newAliasInput.trim()]));
    await safeInvoke('update_work_aliases', { id: workId, aliases: updated });
    setNewAliasInput('');
    setShowAddAlias(false);
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  // Remove alias
  const handleRemoveAlias = async (aliasToRemove: string) => {
    if (!workId) return;
    const updated = aliases.filter((a) => a !== aliasToRemove);
    await safeInvoke('update_work_aliases', { id: workId, aliases: updated });
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  // Archive experience
  const handleArchiveExperience = async (expId: string) => {
    if (!confirm('确定归档此条经历足迹吗？')) return;
    await safeInvoke('archive_experience', { id: expId });
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  // Relocate directory
  const handleRelocateDirectory = async (oldAssetId: string) => {
    const newPath = await safeInvoke<string | null>('pick_folder');
    if (!newPath || !workId) return;
    await safeInvoke('relocate_work_directory', {
      workId,
      oldAssetId,
      newPath,
    });
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  // Add External Reference
  const handleAddRef = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!refUrl.trim() || !workId) return;
    setAddingRef(true);
    try {
      const ref: ExternalReference = {
        id: 'ref_' + Math.random().toString(36).substring(2, 9),
        entity_id: workId,
        provider: refProvider,
        title: refTitle.trim() || refProvider,
        url: refUrl.trim(),
        description: null,
        metadata: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      await safeInvoke('create_external_reference', { reference: ref });
      // Set as primary if no primary yet
      if (!summary?.external_references.length) {
        await safeInvoke('set_primary_external_reference', {
          workId,
          referenceId: ref.id,
        });
      }
      setShowAddRefModal(false);
      setRefTitle('');
      setRefUrl('');
      await loadSummary();
      if (onRefreshAll) await onRefreshAll();
    } finally {
      setAddingRef(false);
    }
  };

  // Delete work
  const handleDeleteWork = async () => {
    if (!confirm(`确定彻底删除作品《${work?.title || ''}》档案吗？所有关联经历与关系也将一并移除。`)) return;
    await safeInvoke('delete_entity', { id: workId });
    if (onRefreshAll) await onRefreshAll();
    onClose();
  };

  // Link asset
  const handleLinkAsset = async (assetId: string) => {
    if (!workId) return;
    const relation = {
      id: 'rel_' + Math.random().toString(36).substring(2, 9),
      source_id: assetId,
      source_type: 'asset',
      relation_type: 'attaches',
      target_id: workId,
      target_type: 'entity',
      metadata: {},
      created_at: new Date().toISOString(),
    };
    await safeInvoke('create_relation', { relation });
    setShowLinkAssetModal(false);
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  // Unlink asset
  const handleUnlinkAsset = async (assetId: string) => {
    if (!workId) return;
    await safeInvoke('delete_relation_between', {
      sourceId: assetId,
      targetId: workId,
    });
    await loadSummary();
    if (onRefreshAll) await onRefreshAll();
  };

  return (
    <>
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-in fade-in">
        <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-3xl max-h-[90vh] flex flex-col shadow-2xl overflow-hidden">
          {/* Header */}
          <div className="p-5 border-b border-neutral-800 flex items-center justify-between bg-neutral-900/60">
            <div className="flex items-center gap-3 min-w-0">
              <span className="p-2 rounded-xl bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">
                <Layers className="w-5 h-5" />
              </span>
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-neutral-800 text-neutral-400">
                    {meta?.work_type || work?.entity_type || 'work'}
                  </span>
                  <h3 className="text-base font-bold text-white truncate">
                    {work?.title || '加载中...'}
                  </h3>
                </div>
                {meta?.original_title && (
                  <p className="text-xs text-neutral-400 mt-0.5 truncate">
                    原名: {meta.original_title}
                  </p>
                )}
              </div>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={handleDeleteWork}
                className="p-1.5 rounded-lg hover:bg-neutral-800 text-neutral-500 hover:text-red-400 transition-colors"
                title="删除作品档案"
              >
                <Trash2 className="w-4 h-4" />
              </button>
              <button
                onClick={onClose}
                className="p-1.5 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
          </div>

          {/* Drawer Body */}
          <div className="flex-1 overflow-y-auto p-6 space-y-6">
            {loading && !summary ? (
              <div className="text-center py-16 text-neutral-500 text-xs">正在加载作品详情...</div>
            ) : summary ? (
              <>
                {errorMsg && (
                  <div className="p-3 rounded-xl bg-red-950/40 border border-red-800 text-xs text-red-300">
                    {errorMsg}
                  </div>
                )}
                {/* Status & Aliases Bar */}
                <div className="flex flex-wrap items-center justify-between gap-3 p-4 rounded-xl bg-neutral-950 border border-neutral-850">
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-neutral-400 font-medium">当前状态:</span>
                    <select
                      value={meta?.status || 'unknown'}
                      onChange={(e) => handleUpdateStatus(e.target.value as RecordStatus)}
                      className="px-2.5 py-1 bg-neutral-900 border border-neutral-800 rounded-lg text-xs font-semibold text-neutral-200 focus:outline-none focus:border-indigo-500 cursor-pointer"
                    >
                      <option value="in_progress">进行中 (In Progress)</option>
                      <option value="planned">想看/想玩 (Planned)</option>
                      <option value="completed">已完成/已通关 (Completed)</option>
                      <option value="paused">搁置 (Paused)</option>
                      <option value="dropped">弃坑 (Dropped)</option>
                      <option value="revisit">重温中 (Revisit)</option>
                      <option value="archived">已归档 (Archived)</option>
                    </select>
                  </div>

                  {/* Aliases */}
                  <div className="flex items-center gap-1.5 flex-wrap">
                    <span className="text-xs text-neutral-500 font-mono">别名:</span>
                    {aliases.map((al) => (
                      <span
                        key={al}
                        className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded bg-neutral-900 text-neutral-300 border border-neutral-800 font-mono"
                      >
                        <span>{al}</span>
                        <button
                          onClick={() => handleRemoveAlias(al)}
                          className="text-neutral-500 hover:text-red-400 ml-0.5"
                        >
                          ×
                        </button>
                      </span>
                    ))}
                    {showAddAlias ? (
                      <div className="inline-flex items-center gap-1">
                        <input
                          type="text"
                          value={newAliasInput}
                          onChange={(e) => setNewAliasInput(e.target.value)}
                          onKeyDown={(e) => e.key === 'Enter' && handleAddAlias()}
                          placeholder="输入别名..."
                          className="px-2 py-0.5 text-xs bg-neutral-900 border border-indigo-500 rounded text-neutral-200 focus:outline-none w-24 font-mono"
                          autoFocus
                        />
                        <button
                          onClick={handleAddAlias}
                          className="px-2 py-0.5 bg-indigo-600 hover:bg-indigo-500 text-white text-[11px] rounded"
                        >
                          添加
                        </button>
                        <button
                          onClick={() => setShowAddAlias(false)}
                          className="text-neutral-500 text-xs px-1"
                        >
                          取消
                        </button>
                      </div>
                    ) : (
                      <button
                        onClick={() => setShowAddAlias(true)}
                        className="text-[11px] px-2 py-0.5 rounded bg-neutral-900 hover:bg-neutral-850 text-indigo-400 border border-neutral-800 flex items-center gap-0.5 transition-colors"
                      >
                        <Plus className="w-3 h-3" />
                        <span>添加别名</span>
                      </button>
                    )}
                  </div>
                </div>

                {/* Progress Tracking Section */}
                <div className="p-4 rounded-xl bg-neutral-950 border border-neutral-850 space-y-2">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-neutral-300 font-semibold flex items-center gap-1.5">
                      <Clock className="w-3.5 h-3.5 text-indigo-400" />
                      <span>进度追踪</span>
                    </span>
                    <span className="text-neutral-400 font-mono">
                      当前进度: {progress?.position || 0}
                      {progress?.total_positions ? ` / ${progress.total_positions}` : ''}{' '}
                      {progress?.unit || ''}
                    </span>
                  </div>

                  <div className="flex items-center gap-3 pt-1">
                    <button
                      onClick={() => handleUpdateProgress((progress?.position || 0) - 1)}
                      className="p-1.5 rounded-lg bg-neutral-900 hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 border border-neutral-800 transition-colors"
                      title="后退 1"
                    >
                      <Minus className="w-3.5 h-3.5" />
                    </button>
                    <input
                      type="range"
                      min={0}
                      max={progress?.total_positions || 100}
                      value={progress?.position || 0}
                      onChange={(e) => handleUpdateProgress(Number(e.target.value))}
                      className="flex-1 accent-indigo-500 h-1.5 bg-neutral-800 rounded-lg cursor-pointer"
                    />
                    <button
                      onClick={() => handleUpdateProgress((progress?.position || 0) + 1)}
                      className="p-1.5 rounded-lg bg-neutral-900 hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 border border-neutral-800 transition-colors"
                      title="前进 1"
                    >
                      <Plus className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>

                {/* Cross-Platform Playtime & Activity Records */}
                {activities.length > 0 && (
                  <div className="space-y-3">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-semibold text-neutral-200 uppercase tracking-wider flex items-center gap-1.5">
                        <Gamepad2 className="w-3.5 h-3.5 text-red-400" />
                        <span>游玩时长与设备记录 ({activities.length})</span>
                      </h4>
                    </div>

                    <div className="space-y-2.5">
                      {activities.map((act) => {
                        const hours = (act.playtime_seconds / 3600).toFixed(1);
                        const peakHours = (act.peak_seen_seconds / 3600).toFixed(1);
                        const isReset = act.peak_seen_seconds > act.playtime_seconds;
                        const lastPlayed = act.last_played_at
                          ? new Date(act.last_played_at).toLocaleDateString()
                          : '未记录';

                        return (
                          <div
                            key={act.id}
                            className="p-3.5 rounded-xl bg-neutral-950 border border-neutral-850 space-y-2.5 text-xs"
                          >
                            <div className="flex items-center justify-between gap-2 flex-wrap">
                              <div className="flex items-center gap-2">
                                <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-red-500/10 text-red-400 border border-red-500/20">
                                  {act.source_label || (act.provider === 'switch_nx' ? 'Switch' : act.provider)}
                                </span>
                                <span className="font-mono text-emerald-400 font-bold text-sm">
                                  {hours} 小时
                                </span>
                                {act.launch_count != null && (
                                  <span className="text-[11px] text-neutral-400">
                                    • {act.launch_count} 次启动
                                  </span>
                                )}
                              </div>

                              <button
                                onClick={() => {
                                  setExpDefaultType('completed');
                                  setExpDefaultPlatform(act.source_label || 'Switch');
                                  setExpDefaultNote(`Switch 累计游玩约 ${hours} 小时`);
                                  setShowExpModal(true);
                                }}
                                className="px-2.5 py-1 rounded-lg bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 font-medium text-[11px] flex items-center gap-1 transition-colors cursor-pointer"
                                title="以当前 Switch 游玩时长为参考，补录通关足迹"
                              >
                                <Compass className="w-3 h-3" />
                                <span>补录通关足迹</span>
                              </button>
                            </div>

                            <div className="flex items-center gap-3 text-[11px] text-neutral-400 flex-wrap">
                              <span>最近游玩: {lastPlayed}</span>
                              <span>•</span>
                              <span className="font-mono text-[10px]">Title ID: {act.external_id}</span>
                            </div>

                            {isReset && (
                              <div className="p-2 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-300 text-[11px] flex items-center gap-1.5">
                                <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                                <span>
                                  当前源报告 {hours}h；历史曾报告 {peakHours}h（可能经历过系统重置）
                                </span>
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}

                {/* Personal Footprints & Experiences (Core V0.5 Highlight) */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h4 className="text-xs font-semibold text-neutral-200 uppercase tracking-wider flex items-center gap-1.5">
                      <Compass className="w-3.5 h-3.5 text-emerald-400" />
                      <span>个人经历与足迹 ({summary.experiences.length})</span>
                    </h4>
                    <button
                      onClick={() => {
                        setExpDefaultType('completed');
                        setShowExpModal(true);
                      }}
                      className="text-xs px-2.5 py-1 rounded-lg bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 font-medium flex items-center gap-1 transition-colors"
                    >
                      <Plus className="w-3 h-3" />
                      <span>记录新足迹</span>
                    </button>
                  </div>

                  {summary.experiences.length === 0 ? (
                    <div className="p-6 text-center rounded-xl bg-neutral-950 border border-neutral-850 space-y-1.5">
                      <p className="text-xs text-neutral-400">暂无此作品的个人足迹记录</p>
                      <p className="text-[11px] text-neutral-500">
                        记录通关日期、多周目重温、游玩平台或专属回忆，永不因文件搬移而丢失。
                      </p>
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {summary.experiences.map((exp) => {
                        const isUnknown = exp.occurred_precision === 'unknown';
                        const ctx = (exp.context_json || {}) as any;
                        return (
                          <div
                            key={exp.id}
                            className="p-3.5 rounded-xl bg-neutral-950 border border-neutral-850 flex items-start justify-between gap-3 text-xs"
                          >
                            <div className="space-y-1 flex-1">
                              <div className="flex items-center gap-2 flex-wrap">
                                <span className="px-2 py-0.5 rounded text-[10px] bg-emerald-500/10 text-emerald-400 font-semibold border border-emerald-500/20">
                                  {exp.event_type === 'completed'
                                    ? '通关达成'
                                    : exp.event_type === 'replayed'
                                    ? '多周目'
                                    : exp.event_type === 'started'
                                    ? '入坑'
                                    : exp.event_type}
                                </span>
                                <span className="font-bold text-neutral-200">
                                  {exp.title || '个人足迹'}
                                </span>
                                {isUnknown ? (
                                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-400 font-mono">
                                    发生时间未知
                                  </span>
                                ) : (
                                  <span className="text-[11px] text-neutral-400 font-mono">
                                    {exp.occurred_value}
                                  </span>
                                )}
                                {ctx.platform && (
                                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-neutral-900 border border-neutral-800 text-sky-400 font-mono">
                                    {ctx.platform}
                                  </span>
                                )}
                              </div>
                              {exp.note && (
                                <p className="text-neutral-400 text-xs leading-relaxed mt-1 pl-1 border-l-2 border-neutral-800">
                                  {exp.note}
                                </p>
                              )}
                            </div>

                            <button
                              onClick={() => handleArchiveExperience(exp.id)}
                              className="text-neutral-500 hover:text-neutral-300 p-1 rounded hover:bg-neutral-900"
                              title="归档此足迹"
                            >
                              <Archive className="w-3.5 h-3.5" />
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>

                {/* Primary Reference & External Links */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h4 className="text-xs font-semibold text-neutral-200 uppercase tracking-wider flex items-center gap-1.5">
                      <ExternalLink className="w-3.5 h-3.5 text-sky-400" />
                      <span>外部权威参考 ({summary.external_references.length})</span>
                    </h4>
                    <button
                      onClick={() => setShowAddRefModal(true)}
                      className="text-xs px-2.5 py-1 rounded-lg bg-sky-500/10 hover:bg-sky-500/20 text-sky-400 border border-sky-500/30 font-medium flex items-center gap-1 transition-colors"
                    >
                      <Plus className="w-3 h-3" />
                      <span>添加外链</span>
                    </button>
                  </div>

                  {summary.external_references.length === 0 ? (
                    <div className="p-4 rounded-xl bg-neutral-950 border border-neutral-850 text-xs text-neutral-500">
                      暂未关联 Steam、Bangumi、VNDB 或官方外链。
                    </div>
                  ) : (
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                      {summary.external_references.map((ref) => (
                        <div
                          key={ref.id}
                          onClick={() => invoke('open_external_url', { url: ref.url })}
                          className="p-3 rounded-xl bg-neutral-950 border border-neutral-850 hover:border-neutral-700 transition-all flex items-center justify-between cursor-pointer group"
                        >
                          <div className="min-w-0 pr-2">
                            <div className="flex items-center gap-1.5 mb-0.5">
                              <span className="text-[10px] font-mono uppercase px-1.5 py-0.2 rounded bg-neutral-900 text-sky-400 border border-neutral-800">
                                {ref.provider}
                              </span>
                              <span className="text-xs font-semibold text-neutral-200 truncate">
                                {ref.title}
                              </span>
                            </div>
                            <p className="text-[11px] text-neutral-500 font-mono truncate">
                              {ref.url}
                            </p>
                          </div>
                          <ExternalLink className="w-3.5 h-3.5 text-neutral-500 group-hover:text-sky-400 shrink-0 transition-colors" />
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Linked Assets & Directory Attachments */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h4 className="text-xs font-semibold text-neutral-200 uppercase tracking-wider flex items-center gap-1.5">
                      <HardDrive className="w-3.5 h-3.5 text-amber-400" />
                      <span>存储目录与关联资产 ({summary.linked_assets.length})</span>
                    </h4>
                    <button
                      onClick={() => setShowLinkAssetModal(true)}
                      className="text-xs px-2.5 py-1 rounded-lg bg-neutral-800 hover:bg-neutral-750 text-neutral-300 border border-neutral-700 font-medium flex items-center gap-1 transition-colors"
                    >
                      <Plus className="w-3 h-3" />
                      <span>关联新资产</span>
                    </button>
                  </div>

                  {summary.linked_assets.length === 0 ? (
                    <div className="p-4 rounded-xl bg-neutral-950 border border-neutral-850 text-xs text-neutral-500">
                      暂无关联文件或媒体。可在下方选择已有资产进行绑定。
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {summary.linked_assets.map((asset) => (
                        <div
                          key={asset.id}
                          className="p-3 rounded-xl bg-neutral-950 border border-neutral-850 flex items-center justify-between gap-3 text-xs"
                        >
                            <div className="flex items-center gap-2.5 min-w-0 flex-1">
                              <span className="p-1.5 rounded-lg bg-neutral-900 text-neutral-400 border border-neutral-800 shrink-0">
                                <Folder className="w-4 h-4 text-amber-400" />
                              </span>
                              <div className="min-w-0">
                                <span className="font-mono text-neutral-300 truncate block">
                                  {asset.path || asset.id}
                                </span>
                                <span className="text-[10px] text-neutral-500 font-mono">
                                  类型: {asset.kind} · 状态: {asset.status}
                                </span>
                              </div>
                            </div>

                            <div className="flex items-center gap-1.5 shrink-0">
                              <button
                                onClick={() => handleRelocateDirectory(asset.id)}
                                className="px-2 py-1 rounded bg-neutral-900 hover:bg-neutral-800 text-neutral-300 text-[11px] border border-neutral-800 transition-colors"
                                title="重定位物理路径"
                              >
                                重定位
                              </button>
                              <button
                                onClick={() => handleUnlinkAsset(asset.id)}
                                className="p-1 rounded text-neutral-500 hover:text-red-400 transition-colors"
                                title="解除关联"
                              >
                                <X className="w-3.5 h-3.5" />
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                  )}
                </div>
              </>
            ) : null}
          </div>
        </div>
      </div>

      {/* Experience Footprint Modal */}
      <ExperienceModal
        isOpen={showExpModal}
        onClose={() => {
          setShowExpModal(false);
          setExpDefaultPlatform('');
          setExpDefaultNote('');
        }}
        workId={workId}
        workTitle={work?.title || ''}
        defaultEventType={expDefaultType}
        defaultPlatform={expDefaultPlatform}
        defaultNote={expDefaultNote}
        onSuccess={async () => {
          await loadSummary();
          if (onRefreshAll) await onRefreshAll();
        }}
      />

      {/* Add Reference Modal */}
      {showAddRefModal && (
        <div className="fixed inset-0 z-[60] bg-black/80 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl p-5 space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-bold text-white flex items-center gap-2">
                <ExternalLink className="w-4 h-4 text-sky-400" />
                <span>添加外部权威参考链接</span>
              </h3>
              <button
                onClick={() => setShowAddRefModal(false)}
                className="text-neutral-500 hover:text-neutral-300"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form onSubmit={handleAddRef} className="space-y-3.5">
              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1">
                  平台 / 提供方
                </label>
                <select
                  value={refProvider}
                  onChange={(e) => setRefProvider(e.target.value)}
                  className="w-full px-3 py-1.5 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 focus:outline-none focus:border-sky-500"
                >
                  <option value="steam">Steam</option>
                  <option value="bangumi">Bangumi (番组计划)</option>
                  <option value="vndb">VNDB</option>
                  <option value="douban">豆瓣 (Douban)</option>
                  <option value="epic">Epic Games</option>
                  <option value="gog">GOG</option>
                  <option value="official">官方网站 (Official)</option>
                  <option value="other">其他</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1">
                  标题 (可选)
                </label>
                <input
                  type="text"
                  placeholder="例如: Steam 商店页 / Bangumi 词条"
                  value={refTitle}
                  onChange={(e) => setRefTitle(e.target.value)}
                  className="w-full px-3 py-1.5 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-sky-500"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1">
                  链接地址 (URL) <span className="text-sky-400">*</span>
                </label>
                <input
                  type="url"
                  required
                  placeholder="https://..."
                  value={refUrl}
                  onChange={(e) => setRefUrl(e.target.value)}
                  className="w-full px-3 py-1.5 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-sky-500 font-mono"
                />
              </div>

              <div className="pt-2 flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setShowAddRefModal(false)}
                  className="px-3 py-1.5 rounded-lg bg-neutral-800 text-neutral-300 text-xs"
                >
                  取消
                </button>
                <button
                  type="submit"
                  disabled={addingRef}
                  className="px-4 py-1.5 rounded-lg bg-sky-500 hover:bg-sky-400 text-neutral-950 text-xs font-bold transition-all disabled:opacity-50"
                >
                  {addingRef ? '添加中...' : '确认添加'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Link Asset Modal */}
      {showLinkAssetModal && (
        <div className="fixed inset-0 z-[60] bg-black/80 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl p-5 space-y-4 max-h-[85vh] flex flex-col">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-bold text-white flex items-center gap-2">
                <FolderOpen className="w-4 h-4 text-amber-400" />
                <span>关联数字资产至作品</span>
              </h3>
              <button
                onClick={() => setShowLinkAssetModal(false)}
                className="text-neutral-500 hover:text-neutral-300"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <input
              type="text"
              placeholder="搜索资产路径或文件名..."
              value={assetSearchQuery}
              onChange={(e) => setAssetSearchQuery(e.target.value)}
              className="w-full px-3 py-1.5 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none font-mono"
            />

            <div className="flex-1 overflow-y-auto space-y-1.5 max-h-64 pr-1">
              {availableAssets
                .filter((a) => {
                  if (!assetSearchQuery.trim()) return true;
                  const q = assetSearchQuery.toLowerCase();
                  return (a.path || '').toLowerCase().includes(q) || a.id.includes(q);
                })
                .slice(0, 50)
                .map((a) => (
                  <div
                    key={a.id}
                    onClick={() => handleLinkAsset(a.id)}
                    className="p-2.5 rounded-lg bg-neutral-950 hover:bg-neutral-850 border border-neutral-850 hover:border-neutral-700 flex items-center justify-between cursor-pointer transition-colors text-xs"
                  >
                    <span className="font-mono text-neutral-300 truncate max-w-sm">
                      {a.path || a.id}
                    </span>
                    <button className="px-2 py-0.5 rounded bg-indigo-600 hover:bg-indigo-500 text-white text-[11px] font-medium">
                      关联
                    </button>
                  </div>
                ))}
            </div>

            <div className="flex justify-end pt-2 border-t border-neutral-800">
              <button
                onClick={() => setShowLinkAssetModal(false)}
                className="px-3 py-1.5 rounded-lg text-neutral-400 hover:text-neutral-200 text-xs"
              >
                关闭
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};

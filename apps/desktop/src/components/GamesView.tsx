import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Gamepad2,
  Plus,
  Search,
  ExternalLink,
  HardDrive,
  Folder,
  FolderOpen,
  CheckCircle2,
  Clock,
  PauseCircle,
  XCircle,
  Sparkles,
  Trash2,
  X,
  Minus,
  RefreshCw,
  FolderSearch,
  Layers,
  AlertCircle,
} from 'lucide-react';
import {
  Entity,
  Asset,
  WorkSummary,
  RecordStatus,
  GameCandidate,
} from '../types';

interface GamesViewProps {
  assets: Asset[];
  onRefreshAll: () => Promise<void> | void;
}

export const GamesView: React.FC<GamesViewProps> = ({
  assets,
  onRefreshAll,
}) => {
  // Game entities (Work where work_type = 'game')
  const [games, setGames] = useState<Entity[]>([]);
  const [loading, setLoading] = useState(false);

  // Filters
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [driveFilter, setDriveFilter] = useState<string>('all');
  const [presenceFilter, setPresenceFilter] = useState<'all' | 'with_local' | 'web_only'>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Active game detail drawer
  const [activeGameId, setActiveGameId] = useState<string | null>(null);
  const [activeSummary, setActiveSummary] = useState<WorkSummary | null>(null);
  const [loadingSummary, setLoadingSummary] = useState(false);

  // Modals
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showCandidateModal, setShowCandidateModal] = useState(false);
  const [showAddRefModal, setShowAddRefModal] = useState(false);
  const [showLinkAssetModal, setShowLinkAssetModal] = useState(false);

  // Create Form State
  const [newTitle, setNewTitle] = useState('');
  const [newOrigTitle, setNewOrigTitle] = useState('');
  const [newStatus, setNewStatus] = useState<RecordStatus>('in_progress');
  const [newDesc, setNewDesc] = useState('');
  const [newLocalPath, setNewLocalPath] = useState('');
  const [creating, setCreating] = useState(false);

  // Candidate Scanner State
  const [scanPath, setScanPath] = useState('D:/Games');
  const [scanningCandidates, setScanningCandidates] = useState(false);
  const [candidates, setCandidates] = useState<GameCandidate[]>([]);
  const [candidateError, setCandidateError] = useState<string | null>(null);

  // Add Ref Form State
  const [refProvider, setRefProvider] = useState('steam');
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

  // Load games list
  const loadGames = useCallback(async () => {
    setLoading(true);
    try {
      const list = await safeInvoke<Entity[]>('list_works', {
        workType: 'game',
        status: statusFilter === 'all' ? null : statusFilter,
      });
      setGames(list || []);
    } finally {
      setLoading(false);
    }
  }, [statusFilter]);

  useEffect(() => {
    loadGames();
  }, [loadGames]);

  // Load active game summary when selected
  useEffect(() => {
    if (!activeGameId) {
      setActiveSummary(null);
      return;
    }
    let isCurrent = true;
    setLoadingSummary(true);
    safeInvoke<WorkSummary>('get_work_summary', { id: activeGameId }).then((summary) => {
      if (isCurrent && summary) {
        setActiveSummary(summary);
      }
      if (isCurrent) setLoadingSummary(false);
    });
    return () => {
      isCurrent = false;
    };
  }, [activeGameId]);

  // Handle open path in file manager
  const handleOpenPath = async (path: string) => {
    await safeInvoke('open_in_file_manager', { path });
  };

  // Handle open external url
  const handleOpenUrl = async (url: string) => {
    await safeInvoke('open_external_url', { url });
  };

  // Quick Progress stepper for games
  const handleQuickStepProgress = async (gameId: string, currentPos: number, delta: number, e: React.MouseEvent) => {
    e.stopPropagation();
    const nextPos = Math.max(0, currentPos + delta);
    await safeInvoke('update_work_progress', {
      id: gameId,
      position: nextPos,
      positionType: 'chapter',
    });
    await loadGames();
    if (activeGameId === gameId) {
      const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: gameId });
      if (updated) setActiveSummary(updated);
    }
    if (onRefreshAll) onRefreshAll();
  };

  // Handle status update
  const handleUpdateStatus = async (gameId: string, newStatus: RecordStatus) => {
    await safeInvoke('update_work_status', { id: gameId, status: newStatus });
    await loadGames();
    if (activeGameId === gameId) {
      const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: gameId });
      if (updated) setActiveSummary(updated);
    }
    if (onRefreshAll) onRefreshAll();
  };

  // Handle cover asset update
  const handleSetCoverAsset = async (gameId: string, assetId: string | null) => {
    await safeInvoke('update_work_cover_asset', { id: gameId, coverAssetId: assetId });
    await loadGames();
    if (activeGameId === gameId) {
      const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: gameId });
      if (updated) setActiveSummary(updated);
    }
  };

  // Handle pick folder for candidate detection
  const handlePickCandidateFolder = async () => {
    const picked = await safeInvoke<string>('pick_folder');
    if (picked) {
      setScanPath(picked);
    }
  };

  // Run Candidate Detection
  const handleRunCandidateScan = async () => {
    if (!scanPath.trim()) return;
    setScanningCandidates(true);
    setCandidateError(null);
    try {
      const res = await safeInvoke<GameCandidate[]>('detect_game_candidates', { rootPath: scanPath });
      setCandidates(res || []);
      if (!res || res.length === 0) {
        setCandidateError(`在目录 "${scanPath}" 下未检测到游戏子目录`);
      }
    } catch (err) {
      setCandidateError(String(err));
    } finally {
      setScanningCandidates(false);
    }
  };

  // Create work from detected candidate
  const handleCreateFromCandidate = async (cand: GameCandidate) => {
    try {
      const newWork = await safeInvoke<Entity>('create_work', {
        title: cand.deduced_title,
        kind: 'game',
        status: 'planned',
        description: `扫描自本地位置: ${cand.path}`,
      });

      if (newWork) {
        // Transactionally ensure Directory Asset and link to work via LoomaCore
        await safeInvoke('link_work_directory', {
          workId: newWork.id,
          path: cand.path,
        });

        // Remove from candidates list
        setCandidates((prev) => prev.filter((c) => c.id !== cand.id));
        await loadGames();
        if (onRefreshAll) onRefreshAll();
      }
    } catch (err) {
      alert(`创建游戏失败: ${err}`);
    }
  };

  // Link candidate to an existing game
  const handleLinkCandidateToGame = async (cand: GameCandidate, targetGameId: string) => {
    try {
      // Transactionally ensure Directory Asset and link to target work via LoomaCore
      await safeInvoke('link_work_directory', {
        workId: targetGameId,
        path: cand.path,
      });

      setCandidates((prev) => prev.filter((c) => c.id !== cand.id));
      await loadGames();
      if (activeGameId === targetGameId) {
        const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: targetGameId });
        if (updated) setActiveSummary(updated);
      }
      if (onRefreshAll) onRefreshAll();
    } catch (err) {
      alert(`关联位置失败: ${err}`);
    }
  };

  // Manual create game
  const handleCreateGame = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;
    setCreating(true);
    try {
      const created = await safeInvoke<Entity>('create_work', {
        title: newTitle.trim(),
        kind: 'game',
        status: newStatus,
        originalTitle: newOrigTitle.trim() || null,
        description: newDesc.trim() || null,
      });

      if (created && newLocalPath.trim()) {
        // Transactionally ensure Directory Asset and link to created work via LoomaCore
        await safeInvoke('link_work_directory', {
          workId: created.id,
          path: newLocalPath.trim(),
        });
      }

      setShowCreateModal(false);
      setNewTitle('');
      setNewOrigTitle('');
      setNewDesc('');
      setNewLocalPath('');
      await loadGames();
      if (onRefreshAll) onRefreshAll();
    } finally {
      setCreating(false);
    }
  };

  // Add external reference
  const handleAddRef = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeGameId || !refUrl.trim()) return;
    setAddingRef(true);
    try {
      await safeInvoke('create_external_reference', {
        reference: {
          id: `ref_${Date.now()}`,
          entity_id: activeGameId,
          provider: refProvider,
          title: refTitle.trim() || refProvider,
          url: refUrl.trim(),
        },
      });
      setShowAddRefModal(false);
      setRefTitle('');
      setRefUrl('');
      const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: activeGameId });
      if (updated) setActiveSummary(updated);
    } finally {
      setAddingRef(false);
    }
  };

  // Link existing asset to active game
  const handleLinkAsset = async (asset: Asset) => {
    if (!activeGameId) return;
    await safeInvoke('create_relation', {
      relation: {
        id: `rel_${Date.now()}`,
        source_id: activeGameId,
        target_id: asset.id,
        relation_type: 'attaches',
        description: asset.path,
      },
    });
    setShowLinkAssetModal(false);
    const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: activeGameId });
    if (updated) setActiveSummary(updated);
  };

  // Remove relation between active game and asset
  const handleUnlinkAsset = async (assetId: string) => {
    if (!activeGameId) return;
    await safeInvoke('delete_relation_between', {
      sourceId: activeGameId,
      targetId: assetId,
    });
    const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: activeGameId });
    if (updated) setActiveSummary(updated);
    if (onRefreshAll) onRefreshAll();
  };

  // Relocate missing directory asset to a new location without losing work
  const handleRelocateAsset = async (oldAssetId: string) => {
    if (!activeGameId) return;
    const picked = await safeInvoke<string>('pick_folder');
    if (picked) {
      const res = await safeInvoke('link_work_directory', {
        workId: activeGameId,
        path: picked,
      });
      if (res) {
        await safeInvoke('delete_relation_between', {
          sourceId: activeGameId,
          targetId: oldAssetId,
        });
        const updated = await safeInvoke<WorkSummary>('get_work_summary', { id: activeGameId });
        if (updated) setActiveSummary(updated);
        await loadGames();
        if (onRefreshAll) onRefreshAll();
      }
    }
  };

  // Extract drives and stats
  const stats = useMemo(() => {
    let planned = 0;
    let inProgress = 0;
    let completed = 0;
    let paused = 0;
    let dropped = 0;

    const driveMap = new Map<string, number>();
    let withLocal = 0;
    let webOnly = 0;

    games.forEach((g) => {
      const meta = g.properties?.work as any;
      const st = meta?.status || g.properties?.status;
      if (st === 'planned') planned++;
      else if (st === 'in_progress') inProgress++;
      else if (st === 'completed') completed++;
      else if (st === 'paused') paused++;
      else if (st === 'dropped') dropped++;

      // Check description or metadata for disk
      const desc = g.description || '';
      const driveMatch = desc.match(/([a-zA-Z]):[/\\]/);
      if (driveMatch) {
        const d = `${driveMatch[1].toUpperCase()}:`;
        driveMap.set(d, (driveMap.get(d) || 0) + 1);
        withLocal++;
      } else {
        webOnly++;
      }
    });

    return {
      total: games.length,
      planned,
      inProgress,
      completed,
      paused,
      dropped,
      withLocal,
      webOnly,
      drives: Array.from(driveMap.entries()).map(([drive, count]) => ({ drive, count })),
    };
  }, [games]);

  // Filtered games list
  const filteredGames = useMemo(() => {
    return games.filter((g) => {
      const meta = g.properties?.work as any;
      const st = meta?.status || g.properties?.status;

      // Status
      if (statusFilter !== 'all' && st !== statusFilter) return false;

      // Search query
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const tMatch = g.title.toLowerCase().includes(q);
        const origMatch = meta?.original_title?.toLowerCase().includes(q);
        const descMatch = g.description?.toLowerCase().includes(q);
        if (!tMatch && !origMatch && !descMatch) return false;
      }

      // Presence filter
      const desc = g.description || '';
      const hasDrive = /[a-zA-Z]:[/\\]/.test(desc);
      if (presenceFilter === 'with_local' && !hasDrive) return false;
      if (presenceFilter === 'web_only' && hasDrive) return false;

      // Drive filter
      if (driveFilter !== 'all') {
        const driveMatch = desc.match(/([a-zA-Z]):[/\\]/);
        if (!driveMatch || `${driveMatch[1].toUpperCase()}:` !== driveFilter) {
          return false;
        }
      }

      return true;
    });
  }, [games, statusFilter, searchQuery, presenceFilter, driveFilter]);

  const getStatusBadge = (status?: string) => {
    switch (status) {
      case 'in_progress':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
            游玩中
          </span>
        );
      case 'completed':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-sky-500/10 text-sky-400 border border-sky-500/20">
            <CheckCircle2 className="w-3 h-3" />
            已通关
          </span>
        );
      case 'planned':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-neutral-800 text-neutral-300 border border-neutral-700">
            <Clock className="w-3 h-3 text-neutral-400" />
            想玩
          </span>
        );
      case 'paused':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
            <PauseCircle className="w-3 h-3" />
            搁置
          </span>
        );
      case 'dropped':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
            <XCircle className="w-3 h-3" />
            弃坑
          </span>
        );
      default:
        return (
          <span className="px-2 py-0.5 rounded-full text-xs bg-neutral-800 text-neutral-400">
            未知
          </span>
        );
    }
  };

  return (
    <div className="flex h-full w-full overflow-hidden select-none bg-neutral-950 text-neutral-100">
      {/* Main Game Library Container */}
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        {/* Top Header & Stat Banner */}
        <div className="p-6 border-b border-neutral-800/80 bg-neutral-900/40 shrink-0">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-5">
            <div>
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400">
                  <Gamepad2 className="w-4 h-4" />
                </div>
                <h1 className="text-xl font-bold tracking-tight text-white">个人游戏库</h1>
                <span className="text-xs px-2.5 py-0.5 rounded-full bg-neutral-800 text-neutral-300 font-mono">
                  {stats.total} 款游戏
                </span>
              </div>
              <p className="text-xs text-neutral-400 mt-1">
                统一记录分散在各大硬盘、Steam及单机目录中的游戏档案，无需搬移原始游戏文件
              </p>
            </div>

            <div className="flex items-center gap-2.5">
              <button
                onClick={() => {
                  setShowCandidateModal(true);
                  if (candidates.length === 0) {
                    handleRunCandidateScan();
                  }
                }}
                className="px-3.5 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 border border-neutral-700 text-xs font-medium flex items-center gap-2 transition-all cursor-pointer shadow-sm"
              >
                <FolderSearch className="w-4 h-4 text-emerald-400" />
                <span>扫描本地游戏候选</span>
              </button>

              <button
                onClick={() => setShowCreateModal(true)}
                className="px-3.5 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-neutral-950 text-xs font-semibold flex items-center gap-1.5 transition-all shadow-md shadow-emerald-500/20 cursor-pointer"
              >
                <Plus className="w-4 h-4" />
                <span>录入新游戏</span>
              </button>
            </div>
          </div>

          {/* Quick Metrics Bar & Disk Distribution */}
          <div className="flex flex-wrap items-center gap-2 pt-2 border-t border-neutral-800/50 text-xs">
            <span className="text-neutral-500 font-medium">状态指标:</span>
            <button
              onClick={() => setStatusFilter('all')}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors ${
                statusFilter === 'all' ? 'bg-neutral-800 text-white' : 'text-neutral-400 hover:text-neutral-200'
              }`}
            >
              全部 ({stats.total})
            </button>
            <button
              onClick={() => setStatusFilter('in_progress')}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors ${
                statusFilter === 'in_progress' ? 'bg-emerald-500/20 text-emerald-300' : 'text-neutral-400 hover:text-emerald-400'
              }`}
            >
              游玩中 ({stats.inProgress})
            </button>
            <button
              onClick={() => setStatusFilter('completed')}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors ${
                statusFilter === 'completed' ? 'bg-sky-500/20 text-sky-300' : 'text-neutral-400 hover:text-sky-400'
              }`}
            >
              已通关 ({stats.completed})
            </button>
            <button
              onClick={() => setStatusFilter('planned')}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors ${
                statusFilter === 'planned' ? 'bg-neutral-700 text-white' : 'text-neutral-400 hover:text-neutral-200'
              }`}
            >
              想玩 ({stats.planned})
            </button>

            <span className="text-neutral-700 mx-1">|</span>

            <span className="text-neutral-500 font-medium">盘符分布:</span>
            {stats.drives.map((d) => (
              <button
                key={d.drive}
                onClick={() => setDriveFilter(driveFilter === d.drive ? 'all' : d.drive)}
                className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors flex items-center gap-1 ${
                  driveFilter === d.drive
                    ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30'
                    : 'bg-neutral-900 hover:bg-neutral-800 text-neutral-400 border border-neutral-800'
                }`}
              >
                <HardDrive className="w-3 h-3 text-neutral-500" />
                <span>{d.drive}</span>
                <span className="text-[10px] text-neutral-500">({d.count})</span>
              </button>
            ))}

            {stats.webOnly > 0 && (
              <button
                onClick={() => setPresenceFilter(presenceFilter === 'web_only' ? 'all' : 'web_only')}
                className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                  presenceFilter === 'web_only'
                    ? 'bg-neutral-800 text-white'
                    : 'text-neutral-500 hover:text-neutral-300'
                }`}
              >
                仅网页/未关联本地 ({stats.webOnly})
              </button>
            )}
          </div>
        </div>

        {/* Filter Controls & Search */}
        <div className="px-6 py-3 border-b border-neutral-800 flex items-center justify-between gap-4 bg-neutral-950/60">
          <div className="relative flex-1 max-w-md">
            <Search className="w-4 h-4 text-neutral-500 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              placeholder="搜索游戏名称、原名或本地存储路径..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-9 pr-4 py-1.5 bg-neutral-900/80 border border-neutral-800 rounded-lg text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 transition-colors"
            />
          </div>

          <div className="flex items-center gap-2 text-xs">
            <button
              onClick={() => setPresenceFilter('all')}
              className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                presenceFilter === 'all' ? 'bg-neutral-800 text-neutral-100' : 'text-neutral-400 hover:text-neutral-200'
              }`}
            >
              全部
            </button>
            <button
              onClick={() => setPresenceFilter('with_local')}
              className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors flex items-center gap-1 ${
                presenceFilter === 'with_local' ? 'bg-neutral-800 text-emerald-400' : 'text-neutral-400 hover:text-neutral-200'
              }`}
            >
              <Folder className="w-3.5 h-3.5" />
              <span>本地已关联</span>
            </button>
            <button
              onClick={() => loadGames()}
              className="p-1.5 rounded-md hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 transition-colors ml-2"
              title="刷新游戏列表"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
            </button>
          </div>
        </div>

        {/* Game Cards Grid */}
        <div className="flex-1 overflow-y-auto p-6">
          {loading && games.length === 0 ? (
            <div className="flex items-center justify-center h-48 text-neutral-500 text-xs">
              正在加载个人游戏库...
            </div>
          ) : filteredGames.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-64 text-center">
              <Gamepad2 className="w-12 h-12 text-neutral-700 mb-3" />
              <h3 className="text-sm font-semibold text-neutral-300 mb-1">未匹配到符合条件的游戏</h3>
              <p className="text-xs text-neutral-500 max-w-sm mb-4">
                你可以点击“扫描本地游戏候选”从本地硬盘自动检测游戏目录，或手动录入新的单机/Steam游戏档案。
              </p>
              <button
                onClick={() => setShowCandidateModal(true)}
                className="px-3.5 py-2 rounded-lg bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30 text-xs font-medium flex items-center gap-2 transition-colors cursor-pointer"
              >
                <FolderSearch className="w-4 h-4" />
                <span>扫描本地游戏候选</span>
              </button>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {filteredGames.map((game) => {
                const meta = game.properties?.work as any;
                const progress = meta?.progress;
                const desc = game.description || '';
                const pathMatch = desc.match(/[a-zA-Z]:[/\\][^,\n\r]+/);
                const localPath = pathMatch ? pathMatch[0].trim() : null;
                const isSelected = activeGameId === game.id;

                return (
                  <div
                    key={game.id}
                    onClick={() => setActiveGameId(game.id)}
                    className={`rounded-xl border p-4 transition-all cursor-pointer relative group flex flex-col justify-between ${
                      isSelected
                        ? 'bg-neutral-900 border-emerald-500/80 shadow-lg shadow-emerald-500/10'
                        : 'bg-neutral-900/60 border-neutral-800/80 hover:border-neutral-700 hover:bg-neutral-900'
                    }`}
                  >
                    <div>
                      {/* Top Bar: Kind Badge & Status */}
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-neutral-800 text-neutral-300 flex items-center gap-1">
                          <Gamepad2 className="w-3 h-3 text-emerald-400" />
                          <span>Game</span>
                        </span>
                        {getStatusBadge(meta?.status || game.properties?.status)}
                      </div>

                      {/* Title & Original Title */}
                      <h3 className="font-semibold text-sm text-neutral-100 group-hover:text-emerald-300 transition-colors line-clamp-1">
                        {game.title}
                      </h3>
                      {meta?.original_title && (
                        <p className="text-xs text-neutral-500 font-sans line-clamp-1 mb-2">
                          {meta.original_title}
                        </p>
                      )}

                      {/* Description / Summary */}
                      {game.description && (
                        <p className="text-xs text-neutral-400 line-clamp-2 leading-relaxed mb-3 mt-1">
                          {game.description}
                        </p>
                      )}
                    </div>

                    {/* Bottom Details & Progress */}
                    <div className="space-y-3 pt-3 border-t border-neutral-800/60 mt-2">
                      {/* Local Path Badge */}
                      {localPath ? (
                        <div className="flex items-center justify-between gap-2 text-xs bg-neutral-950/80 px-2.5 py-1.5 rounded-lg border border-neutral-800/80">
                          <div className="flex items-center gap-1.5 min-w-0 text-neutral-300">
                            <FolderOpen className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                            <span className="font-mono text-[11px] truncate" title={localPath}>
                              {localPath}
                            </span>
                          </div>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleOpenPath(localPath);
                            }}
                            className="text-[11px] text-emerald-400 hover:text-emerald-300 font-medium shrink-0 cursor-pointer"
                            title="在资源管理器中打开目录"
                          >
                            打开
                          </button>
                        </div>
                      ) : (
                        <div className="text-[11px] text-neutral-600 flex items-center gap-1 italic">
                          <ExternalLink className="w-3 h-3" />
                          <span>仅网络记录 / 未绑定本地路径</span>
                        </div>
                      )}

                      {/* Progress Bar & Stepper */}
                      <div className="flex items-center justify-between gap-3 text-xs">
                        <div className="flex-1">
                          <div className="flex justify-between text-[11px] text-neutral-400 mb-1">
                            <span>游玩进度</span>
                            <span className="font-mono font-medium text-emerald-400">
                              {progress ? `${progress.position} ${progress.unit || '章'}` : '0'}
                            </span>
                          </div>
                          <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
                            <div
                              className="bg-emerald-500 h-full rounded-full transition-all"
                              style={{
                                width: `${Math.min(
                                  100,
                                  progress?.total_positions
                                    ? (progress.position / progress.total_positions) * 100
                                    : progress?.position
                                    ? 45
                                    : 0
                                )}%`,
                              }}
                            />
                          </div>
                        </div>

                        {/* Stepper buttons (+ / -) */}
                        <div className="flex items-center gap-1">
                          <button
                            onClick={(e) => handleQuickStepProgress(game.id, progress?.position || 0, -1, e)}
                            className="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white transition-colors"
                            title="进度 -1"
                          >
                            <Minus className="w-3 h-3" />
                          </button>
                          <button
                            onClick={(e) => handleQuickStepProgress(game.id, progress?.position || 0, 1, e)}
                            className="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white transition-colors"
                            title="进度 +1"
                          >
                            <Plus className="w-3 h-3" />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Game Detail Drawer (Right side) */}
      {activeGameId && (
        <div className="w-96 border-l border-neutral-800 bg-neutral-900/90 flex flex-col h-full overflow-hidden shrink-0 shadow-2xl">
          {/* Drawer Header */}
          <div className="p-4 border-b border-neutral-800 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Gamepad2 className="w-4 h-4 text-emerald-400" />
              <span className="font-semibold text-sm text-neutral-200">游戏详情档案</span>
            </div>
            <button
              onClick={() => setActiveGameId(null)}
              className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 transition-colors"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Drawer Body */}
          <div className="flex-1 overflow-y-auto p-5 space-y-6">
            {loadingSummary && !activeSummary ? (
              <div className="flex items-center justify-center h-48 text-neutral-500 text-xs">
                正在加载游戏详情...
              </div>
            ) : activeSummary ? (
              <>
                {/* Title & Status Form */}
                <div>
                  <h2 className="text-base font-bold text-white mb-1">
                    {activeSummary.entity.title}
                  </h2>
                  {activeSummary.work_metadata?.original_title && (
                    <p className="text-xs text-neutral-500 mb-3">
                      {activeSummary.work_metadata.original_title}
                    </p>
                  )}

                  {/* Status Dropdown */}
                  <div className="flex items-center gap-2 mt-3">
                    <span className="text-xs text-neutral-400">当前状态:</span>
                    <select
                      value={activeSummary.work_metadata?.status || 'in_progress'}
                      onChange={(e) => handleUpdateStatus(activeSummary.entity.id, e.target.value as RecordStatus)}
                      className="px-2.5 py-1 bg-neutral-800 border border-neutral-700 rounded text-xs text-neutral-200 focus:outline-none focus:border-emerald-500"
                    >
                      <option value="planned">想玩 (Planned)</option>
                      <option value="in_progress">游玩中 (In Progress)</option>
                      <option value="completed">已通关 (Completed)</option>
                      <option value="paused">搁置中 (Paused)</option>
                      <option value="dropped">弃坑 (Dropped)</option>
                    </select>
                  </div>
                </div>

                {/* Local Storage & Multi-Location Section */}
                <div className="p-4 rounded-xl bg-neutral-950/70 border border-neutral-800 space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
                      <HardDrive className="w-3.5 h-3.5 text-emerald-400" />
                      <span>本地存储位置与文件</span>
                    </span>
                    <button
                      onClick={() => setShowLinkAssetModal(true)}
                      className="text-[11px] text-emerald-400 hover:text-emerald-300 font-medium flex items-center gap-1 cursor-pointer"
                    >
                      <Plus className="w-3 h-3" />
                      <span>关联位置</span>
                    </button>
                  </div>

                  {activeSummary.linked_assets.length === 0 ? (
                    <div className="text-xs text-neutral-500 italic py-2">
                      暂未关联本地游戏目录或资产文件
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {activeSummary.linked_assets.map((ast) => {
                        const isMissing = ast.status === 'missing';
                        return (
                          <div
                            key={ast.id}
                            className={`p-2.5 rounded-lg flex items-center justify-between gap-2 text-xs transition-colors ${
                              isMissing
                                ? 'bg-amber-950/20 border border-amber-800/60'
                                : 'bg-neutral-900 border border-neutral-800'
                            }`}
                          >
                            <div className="min-w-0 flex-1">
                              <div className="flex items-center gap-1.5 font-mono text-[11px] truncate">
                                {isMissing ? (
                                  <AlertCircle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                                ) : (
                                  <Folder className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                                )}
                                <span
                                  className={`truncate ${
                                    isMissing ? 'text-amber-200 line-through decoration-amber-500/50' : 'text-neutral-200'
                                  }`}
                                >
                                  {ast.path || ast.id}
                                </span>
                                {isMissing && (
                                  <span className="px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300 text-[10px] font-sans font-medium border border-amber-500/30 shrink-0">
                                    目录失效 / 离线
                                  </span>
                                )}
                              </div>
                              {isMissing ? (
                                <div className="text-[10px] text-amber-400/80 mt-0.5">
                                  此本地路径不可访问或移动硬盘未连接，作品及游玩进度不受影响
                                </div>
                              ) : ast.size ? (
                                <div className="text-[10px] text-neutral-500 mt-0.5">
                                  {(ast.size / (1024 * 1024 * 1024)).toFixed(2)} GB
                                </div>
                              ) : null}
                            </div>

                            <div className="flex items-center gap-1.5 shrink-0">
                              {isMissing ? (
                                <button
                                  onClick={() => handleRelocateAsset(ast.id)}
                                  className="px-2 py-0.5 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 text-[11px] border border-amber-500/40 cursor-pointer font-medium transition-colors"
                                  title="选择新的有效目录重新关联"
                                >
                                  重新定位
                                </button>
                              ) : (
                                <>
                                  {ast.kind === 'image' && (
                                    <button
                                      onClick={() => handleSetCoverAsset(activeSummary.entity.id, ast.id)}
                                      className="px-2 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-emerald-400 text-[11px] cursor-pointer"
                                      title="设为此游戏卡片封面"
                                    >
                                      设为封面
                                    </button>
                                  )}
                                  {ast.path && (
                                    <button
                                      onClick={() => handleOpenPath(ast.path!)}
                                      className="px-2 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-[11px] cursor-pointer"
                                      title="打开文件资源管理器"
                                    >
                                      打开
                                    </button>
                                  )}
                                </>
                              )}
                              <button
                                onClick={() => handleUnlinkAsset(ast.id)}
                                className="p-1 rounded hover:bg-neutral-800 text-neutral-500 hover:text-rose-400 transition-colors"
                                title="解除关联"
                              >
                                <Trash2 className="w-3.5 h-3.5" />
                              </button>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>

                {/* External References (Steam, Wiki, Bangumi) */}
                <div className="p-4 rounded-xl bg-neutral-950/70 border border-neutral-800 space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
                      <ExternalLink className="w-3.5 h-3.5 text-sky-400" />
                      <span>外部链接与平台</span>
                    </span>
                    <button
                      onClick={() => setShowAddRefModal(true)}
                      className="text-[11px] text-sky-400 hover:text-sky-300 font-medium flex items-center gap-1 cursor-pointer"
                    >
                      <Plus className="w-3 h-3" />
                      <span>添加外链</span>
                    </button>
                  </div>

                  {activeSummary.external_references.length === 0 ? (
                    <div className="text-xs text-neutral-500 italic py-2">
                      暂无外部参考链接 (支持 Steam, Bangumi, 官网等)
                    </div>
                  ) : (
                    <div className="space-y-1.5">
                      {activeSummary.external_references.map((ref) => (
                        <div
                          key={ref.id}
                          onClick={() => handleOpenUrl(ref.url)}
                          className="p-2 rounded bg-neutral-900 border border-neutral-800/80 hover:border-neutral-700 flex items-center justify-between text-xs cursor-pointer group"
                        >
                          <div className="flex items-center gap-2 min-w-0">
                            <span className="px-1.5 py-0.5 rounded text-[10px] bg-sky-500/10 text-sky-400 uppercase font-mono">
                              {ref.provider}
                            </span>
                            <span className="text-neutral-300 group-hover:text-white truncate">
                              {ref.title}
                            </span>
                          </div>
                          <ExternalLink className="w-3 h-3 text-neutral-500 group-hover:text-sky-400" />
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Linked Memories / Notes */}
                <div className="p-4 rounded-xl bg-neutral-950/70 border border-neutral-800 space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
                      <Layers className="w-3.5 h-3.5 text-purple-400" />
                      <span>游玩手记与心得</span>
                    </span>
                    <span className="text-[10px] text-neutral-500">
                      {activeSummary.linked_memories.length} 篇
                    </span>
                  </div>

                  {activeSummary.linked_memories.length === 0 ? (
                    <div className="text-xs text-neutral-500 italic py-2">
                      暂无通关评价或游玩碎碎念
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {activeSummary.linked_memories.map((mem) => (
                        <div
                          key={mem.id}
                          className="p-2.5 rounded-lg bg-neutral-900 border border-neutral-800 text-xs"
                        >
                          <div className="font-medium text-neutral-200 mb-1">{mem.title}</div>
                          <p className="text-neutral-400 line-clamp-2 text-[11px] leading-relaxed">
                            {mem.content}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </>
            ) : null}
          </div>
        </div>
      )}

      {/* Candidate Detection Modal */}
      {showCandidateModal && (
        <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-2xl overflow-hidden shadow-2xl flex flex-col max-h-[85vh]">
            <div className="p-5 border-b border-neutral-800 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-7 h-7 rounded-lg bg-emerald-500/10 flex items-center justify-center text-emerald-400">
                  <FolderSearch className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="font-bold text-sm text-white">扫描本地游戏候选 (Candidate Detection)</h3>
                  <p className="text-xs text-neutral-400 mt-0.5">
                    仅提供识别建议，绝不擅自批量向数据库写入未确认的垃圾条目
                  </p>
                </div>
              </div>
              <button
                onClick={() => setShowCandidateModal(false)}
                className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Folder Input & Scan trigger */}
            <div className="p-5 border-b border-neutral-800/60 bg-neutral-950/50">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={scanPath}
                  onChange={(e) => setScanPath(e.target.value)}
                  placeholder="例如: D:/Games 或 E:/SteamLibrary/steamapps/common"
                  className="flex-1 px-3 py-2 bg-neutral-900 border border-neutral-700 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 font-mono"
                />
                <button
                  type="button"
                  onClick={handlePickCandidateFolder}
                  className="px-3 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium border border-neutral-700 transition-colors"
                >
                  浏览...
                </button>
                <button
                  type="button"
                  onClick={handleRunCandidateScan}
                  disabled={scanningCandidates}
                  className="px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-neutral-950 text-xs font-bold transition-all disabled:opacity-50 flex items-center gap-1.5"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${scanningCandidates ? 'animate-spin' : ''}`} />
                  <span>{scanningCandidates ? '扫描中...' : '开始扫描'}</span>
                </button>
              </div>

              {candidateError && (
                <div className="flex items-center gap-1.5 text-xs text-amber-400 mt-2.5">
                  <AlertCircle className="w-3.5 h-3.5 shrink-0" />
                  <span>{candidateError}</span>
                </div>
              )}
            </div>

            {/* Candidate List */}
            <div className="flex-1 overflow-y-auto p-5 space-y-3">
              {candidates.length === 0 ? (
                <div className="text-center py-12 text-xs text-neutral-500">
                  {scanningCandidates ? '正在分析子目录特征与执行文件...' : '输入游戏根目录并点击开始扫描'}
                </div>
              ) : (
                candidates.map((cand) => (
                  <div
                    key={cand.id}
                    className="p-3.5 rounded-xl bg-neutral-950 border border-neutral-800/90 flex items-center justify-between gap-3 text-xs"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-bold text-neutral-200 text-sm">{cand.deduced_title}</span>
                        {cand.has_executable && (
                          <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono">
                            .exe 就绪
                          </span>
                        )}
                        <span className="px-1.5 py-0.5 rounded text-[10px] bg-neutral-800 text-neutral-400 font-mono">
                          {cand.drive}
                        </span>
                      </div>
                      <div className="text-neutral-500 font-mono text-[11px] truncate" title={cand.path}>
                        {cand.path}
                      </div>

                      {/* Duplicate / Match Hint */}
                      {cand.matched_work_title && (
                        <div className="mt-1.5 flex items-center gap-1 text-[11px] text-sky-400">
                          <Sparkles className="w-3 h-3" />
                          <span>已匹配现有游戏档案: 《{cand.matched_work_title}》</span>
                        </div>
                      )}
                    </div>

                    {/* Candidate Actions */}
                    <div className="flex items-center gap-2 shrink-0">
                      {cand.matched_work_id ? (
                        <button
                          onClick={() => handleLinkCandidateToGame(cand, cand.matched_work_id!)}
                          className="px-3 py-1.5 rounded-lg bg-sky-500/20 hover:bg-sky-500/30 text-sky-300 font-medium text-xs transition-colors"
                        >
                          关联为附加位置
                        </button>
                      ) : (
                        <button
                          onClick={() => handleCreateFromCandidate(cand)}
                          className="px-3 py-1.5 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-neutral-950 font-bold text-xs transition-all shadow-sm"
                        >
                          建立游戏档案
                        </button>
                      )}
                      <button
                        onClick={() => setCandidates((prev) => prev.filter((c) => c.id !== cand.id))}
                        className="p-1.5 rounded hover:bg-neutral-800 text-neutral-500 hover:text-neutral-300 transition-colors"
                        title="忽略此候选"
                      >
                        <X className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}

      {/* Manual Create Game Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl">
            <div className="p-5 border-b border-neutral-800 flex items-center justify-between">
              <h3 className="font-bold text-sm text-white flex items-center gap-2">
                <Gamepad2 className="w-4 h-4 text-emerald-400" />
                <span>录入新游戏档案</span>
              </h3>
              <button
                onClick={() => setShowCreateModal(false)}
                className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form onSubmit={handleCreateGame} className="p-5 space-y-4">
              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  游戏名称 <span className="text-emerald-400">*</span>
                </label>
                <input
                  type="text"
                  required
                  placeholder="例如: NieR:Automata"
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  原名 / 别名 (可选)
                </label>
                <input
                  type="text"
                  placeholder="例如: ニーア オートマタ"
                  value={newOrigTitle}
                  onChange={(e) => setNewOrigTitle(e.target.value)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  游玩状态
                </label>
                <select
                  value={newStatus}
                  onChange={(e) => setNewStatus(e.target.value as RecordStatus)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 focus:outline-none focus:border-emerald-500"
                >
                  <option value="in_progress">游玩中 (In Progress)</option>
                  <option value="planned">想玩 (Planned)</option>
                  <option value="completed">已通关 (Completed)</option>
                  <option value="paused">搁置 (Paused)</option>
                  <option value="dropped">弃坑 (Dropped)</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  本地存储目录 (可选)
                </label>
                <input
                  type="text"
                  placeholder="例如: E:\Games\NieR_Automata"
                  value={newLocalPath}
                  onChange={(e) => setNewLocalPath(e.target.value)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 font-mono"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  简介与备忘
                </label>
                <textarea
                  rows={3}
                  placeholder="游戏简介、游玩平台、攻略备忘..."
                  value={newDesc}
                  onChange={(e) => setNewDesc(e.target.value)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 resize-none"
                />
              </div>

              <div className="pt-2 flex justify-end gap-2.5">
                <button
                  type="button"
                  onClick={() => setShowCreateModal(false)}
                  className="px-4 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium transition-colors"
                >
                  取消
                </button>
                <button
                  type="submit"
                  disabled={creating}
                  className="px-5 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-neutral-950 text-xs font-bold transition-all disabled:opacity-50"
                >
                  {creating ? '创建中...' : '确认创建'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Add External Reference Modal */}
      {showAddRefModal && (
        <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-md overflow-hidden shadow-2xl">
            <div className="p-5 border-b border-neutral-800 flex items-center justify-between">
              <h3 className="font-bold text-sm text-white">添加外部链接与平台</h3>
              <button
                onClick={() => setShowAddRefModal(false)}
                className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form onSubmit={handleAddRef} className="p-5 space-y-4">
              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  平台 / 提供方
                </label>
                <select
                  value={refProvider}
                  onChange={(e) => setRefProvider(e.target.value)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 focus:outline-none focus:border-emerald-500"
                >
                  <option value="steam">Steam</option>
                  <option value="bangumi">Bangumi</option>
                  <option value="vndb">VNDB</option>
                  <option value="epic">Epic Games</option>
                  <option value="gog">GOG</option>
                  <option value="official">官方网站 (Official)</option>
                  <option value="wiki">百科 / Wiki</option>
                  <option value="other">其他</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  链接标题 (可选)
                </label>
                <input
                  type="text"
                  placeholder="例如: Steam 商店页"
                  value={refTitle}
                  onChange={(e) => setRefTitle(e.target.value)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 focus:outline-none focus:border-emerald-500"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                  链接地址 (URL) <span className="text-emerald-400">*</span>
                </label>
                <input
                  type="url"
                  required
                  placeholder="https://..."
                  value={refUrl}
                  onChange={(e) => setRefUrl(e.target.value)}
                  className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 focus:outline-none focus:border-emerald-500 font-mono"
                />
              </div>

              <div className="pt-2 flex justify-end gap-2.5">
                <button
                  type="button"
                  onClick={() => setShowAddRefModal(false)}
                  className="px-4 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium"
                >
                  取消
                </button>
                <button
                  type="submit"
                  disabled={addingRef}
                  className="px-5 py-2 rounded-lg bg-sky-500 hover:bg-sky-400 text-neutral-950 text-xs font-bold transition-all disabled:opacity-50"
                >
                  {addingRef ? '添加中...' : '添加外链'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Link Asset Modal */}
      {showLinkAssetModal && (
        <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl flex flex-col max-h-[80vh]">
            <div className="p-5 border-b border-neutral-800 flex items-center justify-between">
              <h3 className="font-bold text-sm text-white">从数字资产库关联本地位置</h3>
              <button
                onClick={() => setShowLinkAssetModal(false)}
                className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-4 border-b border-neutral-800">
              <input
                type="text"
                placeholder="搜索资产路径或文件名..."
                value={assetSearch}
                onChange={(e) => setAssetSearch(e.target.value)}
                className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500"
              />
            </div>

            <div className="flex-1 overflow-y-auto p-4 space-y-2">
              {assets
                .filter((a) =>
                  assetSearch
                    ? a.path?.toLowerCase().includes(assetSearch.toLowerCase())
                    : true
                )
                .slice(0, 30)
                .map((ast) => (
                  <div
                    key={ast.id}
                    onClick={() => handleLinkAsset(ast)}
                    className="p-3 rounded-lg bg-neutral-950 border border-neutral-800 hover:border-emerald-500 flex items-center justify-between text-xs cursor-pointer group"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="text-neutral-200 font-mono text-[11px] truncate">
                        {ast.path || ast.id}
                      </div>
                      <div className="text-[10px] text-neutral-500 uppercase mt-0.5">
                        {ast.kind}
                      </div>
                    </div>
                    <span className="text-emerald-400 text-xs font-medium opacity-0 group-hover:opacity-100 transition-opacity">
                      绑定
                    </span>
                  </div>
                ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

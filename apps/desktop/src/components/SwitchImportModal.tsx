import React, { useState, useMemo, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Gamepad2,
  X,
  FileText,
  AlertTriangle,
  CheckCircle2,
  Sparkles,
  Search,
  RefreshCw,
  Plus,
} from 'lucide-react';
import {
  Entity,
  SwitchImportPreview,
  SwitchGameCandidatePreview,
  SwitchImportDecision,
  SwitchImportAction,
  SwitchImportOutcome,
} from '../types';

interface SwitchImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: () => Promise<void> | void;
}

export const SwitchImportModal: React.FC<SwitchImportModalProps> = ({
  isOpen,
  onClose,
  onSuccess,
}) => {
  const [filePath, setFilePath] = useState<string | null>(null);
  const [rawJson, setRawJson] = useState<string | null>(null);
  const [preview, setPreview] = useState<SwitchImportPreview | null>(null);
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);

  // Available games in Vault for manual link selection
  const [vaultGames, setVaultGames] = useState<Entity[]>([]);

  // Per-game action decisions: key is external_id (Title ID)
  // Value: { type: 'CreateNewWork', title } | { type: 'LinkExistingWork', work_id } | { type: 'Skip' }
  const [decisions, setDecisions] = useState<Record<string, SwitchImportAction>>({});

  // Search & filter in candidate table
  const [searchQuery, setSearchQuery] = useState('');
  const [filterMode, setFilterMode] = useState<'all' | 'new' | 'matched' | 'warning'>('all');

  // Loading & submission state
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<SwitchImportOutcome | null>(null);

  // Load existing Vault games for mapping dropdowns
  useEffect(() => {
    if (isOpen) {
      invoke<Entity[]>('list_works', { workType: 'game' })
        .then((res) => setVaultGames(res || []))
        .catch((err) => console.warn('Failed to load vault games:', err));
    }
  }, [isOpen]);

  // Reset state when modal opens or closes
  useEffect(() => {
    if (!isOpen) {
      setFilePath(null);
      setRawJson(null);
      setPreview(null);
      setSelectedUserId(null);
      setDecisions({});
      setErrorMsg(null);
      setOutcome(null);
      setSearchQuery('');
    }
  }, [isOpen]);

  if (!isOpen) return null;

  // 1. Pick and preview JSON file
  const handleSelectFile = async () => {
    setErrorMsg(null);
    setOutcome(null);
    try {
      const path = await invoke<string | null>('pick_json_file');
      if (!path) return;

      setFilePath(path);
      setLoading(true);

      const content = await invoke<string>('read_file_content', { path });
      setRawJson(content);

      const prev = await invoke<SwitchImportPreview>('preview_switch_import', {
        jsonContent: content,
        userId: null,
      });

      setPreview(prev);
      setSelectedUserId(prev.selected_user_id);
      initializeDecisions(prev.games);
    } catch (err: any) {
      console.error('Failed to parse Switch JSON:', err);
      setErrorMsg(typeof err === 'string' ? err : err?.message || '读取或解析文件失败');
    } finally {
      setLoading(false);
    }
  };

  // Switch selected profile if multi-profile
  const handleSwitchProfile = async (userId: string) => {
    if (!rawJson || userId === selectedUserId) return;
    setLoading(true);
    setErrorMsg(null);
    try {
      const prev = await invoke<SwitchImportPreview>('preview_switch_import', {
        jsonContent: rawJson,
        userId,
      });
      setPreview(prev);
      setSelectedUserId(userId);
      initializeDecisions(prev.games);
    } catch (err: any) {
      setErrorMsg(typeof err === 'string' ? err : '切换用户档案失败');
    } finally {
      setLoading(false);
    }
  };

  // Default decisions for each candidate game
  const initializeDecisions = (games: SwitchGameCandidatePreview[]) => {
    const init: Record<string, SwitchImportAction> = {};
    for (const g of games) {
      if (g.match_status === 'ExactMapped' && g.matched_work_id) {
        init[g.external_id] = { type: 'LinkExistingWork', work_id: g.matched_work_id };
      } else if (g.match_status === 'SuggestedMatch' && g.matched_work_id) {
        init[g.external_id] = { type: 'LinkExistingWork', work_id: g.matched_work_id };
      } else {
        init[g.external_id] = { type: 'CreateNewWork', title: g.title_name };
      }
    }
    setDecisions(init);
  };

  // Update a single game's action
  const handleSetAction = (externalId: string, action: SwitchImportAction) => {
    setDecisions((prev) => ({
      ...prev,
      [externalId]: action,
    }));
  };

  // Bulk actions
  const handleBulkSetAll = (type: 'CreateNewWork' | 'Skip') => {
    if (!preview) return;
    const next: Record<string, SwitchImportAction> = { ...decisions };
    for (const g of preview.games) {
      if (type === 'CreateNewWork') {
        next[g.external_id] = { type: 'CreateNewWork', title: g.title_name };
      } else {
        next[g.external_id] = { type: 'Skip' };
      }
    }
    setDecisions(next);
  };

  // Filtered candidate list
  const filteredCandidates = useMemo(() => {
    if (!preview) return [];
    return preview.games.filter((g) => {
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const tMatch = g.title_name.toLowerCase().includes(q);
        const idMatch = g.external_id.toLowerCase().includes(q);
        const workMatch = g.matched_work_title?.toLowerCase().includes(q);
        if (!tMatch && !idMatch && !workMatch) return false;
      }

      if (filterMode === 'new') return g.match_status === 'NewGame';
      if (filterMode === 'matched') return g.match_status !== 'NewGame';
      if (filterMode === 'warning') return !!g.warning || g.is_time_decreased;

      return true;
    });
  }, [preview, searchQuery, filterMode]);

  // Decision counts
  const decisionCounts = useMemo(() => {
    let toCreate = 0;
    let toLink = 0;
    let toSkip = 0;
    for (const val of Object.values(decisions)) {
      if (val.type === 'CreateNewWork') toCreate++;
      else if (val.type === 'LinkExistingWork') toLink++;
      else if (val.type === 'Skip') toSkip++;
    }
    return { toCreate, toLink, toSkip, total: Object.keys(decisions).length };
  }, [decisions]);

  // Submit and commit import
  const handleSubmit = async () => {
    if (!preview || !selectedUserId) return;
    setSubmitting(true);
    setErrorMsg(null);

    try {
      const items = preview.games.map((g) => ({
        external_id: g.external_id,
        title_hint: g.title_name,
        playtime_seconds: g.playtime_seconds,
        launch_count: g.launches ?? null,
        first_played_at: g.first_played_at ?? null,
        last_played_at: g.last_played_at ?? null,
        action: decisions[g.external_id] || { type: 'Skip' },
      }));

      const decision: SwitchImportDecision = {
        file_sha256: preview.file_sha256,
        selected_user_id: selectedUserId,
        user_label: `Switch (${preview.selected_user_name})`,
        source_exported_at: preview.export_timestamp ?? null,
        items,
      };

      const res = await invoke<SwitchImportOutcome>('commit_switch_import', { decision });
      setOutcome(res);
      if (onSuccess) await onSuccess();
    } catch (err: any) {
      console.error('Switch import commit failed:', err);
      setErrorMsg(typeof err === 'string' ? err : err?.message || '导入事务提交失败');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-4 animate-in fade-in">
      <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-4xl max-h-[90vh] shadow-2xl flex flex-col overflow-hidden text-neutral-100">
        {/* Header */}
        <div className="px-6 py-4 border-b border-neutral-800/80 bg-neutral-950/60 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-red-500/15 border border-red-500/30 flex items-center justify-center text-red-400">
              <Gamepad2 className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-bold text-white">同步 Switch 游玩记录</h3>
                <span className="text-[10px] px-2 py-0.5 rounded-full bg-red-500/10 text-red-400 font-mono border border-red-500/20">
                  NX Activity Log JSON
                </span>
              </div>
              <p className="text-xs text-neutral-400 mt-0.5">
                读取 Switch 导出的游玩时长事实，关联或新建作品档案，不修改原游戏与导出文件
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto p-6 space-y-5">
          {/* File Picker Banner */}
          {!preview ? (
            <div className="border-2 border-dashed border-neutral-800 hover:border-red-500/40 rounded-2xl p-10 text-center flex flex-col items-center justify-center gap-3 transition-colors bg-neutral-950/40">
              <div className="w-12 h-12 rounded-2xl bg-red-500/10 flex items-center justify-center text-red-400">
                <FileText className="w-6 h-6" />
              </div>
              <div>
                <h4 className="text-sm font-semibold text-neutral-200">选择 NX Activity Log 导出 JSON</h4>
                <p className="text-xs text-neutral-400 mt-1 max-w-md">
                  由 Switch 端 NX-Activity-Log 插件导出的游玩记录文件，包含各游戏累计秒数、启动次数与最近运行时间。
                </p>
              </div>
              <button
                onClick={handleSelectFile}
                disabled={loading}
                className="mt-2 px-5 py-2.5 rounded-xl bg-red-600 hover:bg-red-500 text-white font-semibold text-xs transition-all shadow-lg shadow-red-600/20 flex items-center gap-2 cursor-pointer disabled:opacity-50"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
                <span>{loading ? '解析中...' : '浏览选择 JSON 文件'}</span>
              </button>
            </div>
          ) : (
            <div className="space-y-4">
              {/* File & Profile Banner */}
              <div className="p-4 rounded-xl bg-neutral-950 border border-neutral-800 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 text-xs">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-semibold text-neutral-200">文件来源:</span>
                    <span className="font-mono text-neutral-400 truncate max-w-xs">{filePath}</span>
                    <button
                      onClick={handleSelectFile}
                      className="text-[11px] text-red-400 hover:underline ml-1"
                    >
                      重新选择
                    </button>
                  </div>
                  <div className="flex items-center gap-3 text-[11px] text-neutral-400">
                    <span>
                      导出时间: {preview.export_timestamp ? new Date(preview.export_timestamp * 1000).toLocaleString() : '未知'}
                    </span>
                    <span>•</span>
                    <span>SHA-256: {preview.file_sha256.substring(0, 12)}...</span>
                  </div>
                </div>

                {/* Profile Selector if multiple */}
                {preview.profiles.length > 1 && (
                  <div className="flex items-center gap-2">
                    <span className="text-neutral-400">选择账号:</span>
                    <div className="flex gap-1.5">
                      {preview.profiles.map((p) => (
                        <button
                          key={p.user_id}
                          onClick={() => handleSwitchProfile(p.user_id)}
                          className={`px-2.5 py-1 rounded-lg text-xs font-medium transition-all ${
                            p.user_id === selectedUserId
                              ? 'bg-red-500/20 text-red-400 border border-red-500/40'
                              : 'bg-neutral-850 text-neutral-400 hover:text-neutral-200'
                          }`}
                        >
                          {p.user_name} ({p.title_count} 款)
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>

              {/* Outcome Banner on Success */}
              {outcome && (
                <div className="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/30 flex items-start gap-3 text-xs text-emerald-300">
                  <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
                  <div>
                    <h5 className="font-bold text-emerald-200">导入完成！</h5>
                    <p className="mt-0.5">
                      成功同步 <strong>{outcome.imported_records}</strong> 条记录，新建 <strong>{outcome.created_works}</strong> 款作品，更新 <strong>{outcome.updated_records}</strong> 条既有记录，跳过 <strong>{outcome.skipped_records}</strong> 条。
                    </p>
                  </div>
                </div>
              )}

              {/* Quick Filter Bar */}
              <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-xs">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-neutral-400">筛选视图:</span>
                  <button
                    onClick={() => setFilterMode('all')}
                    className={`px-2.5 py-1 rounded-lg transition-colors ${
                      filterMode === 'all' ? 'bg-neutral-800 text-white' : 'text-neutral-400 hover:text-neutral-200'
                    }`}
                  >
                    全部 ({preview.total_games})
                  </button>
                  <button
                    onClick={() => setFilterMode('new')}
                    className={`px-2.5 py-1 rounded-lg transition-colors ${
                      filterMode === 'new' ? 'bg-purple-500/20 text-purple-300' : 'text-neutral-400 hover:text-purple-400'
                    }`}
                  >
                    新游戏 ({preview.new_games_count})
                  </button>
                  <button
                    onClick={() => setFilterMode('matched')}
                    className={`px-2.5 py-1 rounded-lg transition-colors ${
                      filterMode === 'matched' ? 'bg-sky-500/20 text-sky-300' : 'text-neutral-400 hover:text-sky-400'
                    }`}
                  >
                    已匹配/关联 ({preview.matched_games_count})
                  </button>
                  {preview.warning_count > 0 && (
                    <button
                      onClick={() => setFilterMode('warning')}
                      className={`px-2.5 py-1 rounded-lg transition-colors ${
                        filterMode === 'warning' ? 'bg-amber-500/20 text-amber-300' : 'text-neutral-400 hover:text-amber-400'
                      }`}
                    >
                      注意项 ({preview.warning_count})
                    </button>
                  )}
                </div>

                <div className="flex items-center gap-2 w-full sm:w-auto">
                  <div className="relative flex-1 sm:w-48">
                    <Search className="w-3.5 h-3.5 text-neutral-500 absolute left-2.5 top-1/2 -translate-y-1/2" />
                    <input
                      type="text"
                      placeholder="搜索游戏或 Title ID..."
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      className="w-full pl-8 pr-3 py-1 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-red-500"
                    />
                  </div>
                  <button
                    onClick={() => handleBulkSetAll('CreateNewWork')}
                    className="px-2 py-1 rounded bg-neutral-850 hover:bg-neutral-800 text-neutral-300 text-[11px] whitespace-nowrap"
                    title="将当前未绑定的游戏均设为新建作品"
                  >
                    全部新建
                  </button>
                  <button
                    onClick={() => handleBulkSetAll('Skip')}
                    className="px-2 py-1 rounded bg-neutral-850 hover:bg-neutral-800 text-neutral-400 text-[11px] whitespace-nowrap"
                    title="将所有游戏设为跳过"
                  >
                    全部跳过
                  </button>
                </div>
              </div>

              {/* Candidate Table */}
              <div className="border border-neutral-800 rounded-xl overflow-hidden bg-neutral-950/60 max-h-[46vh] overflow-y-auto">
                <table className="w-full text-left text-xs border-collapse">
                  <thead className="bg-neutral-900/90 text-neutral-400 text-[11px] sticky top-0 z-10 border-b border-neutral-800">
                    <tr>
                      <th className="py-2.5 px-3 font-medium">游戏名称 / Title ID</th>
                      <th className="py-2.5 px-3 font-medium">游玩时长</th>
                      <th className="py-2.5 px-3 font-medium">启动次数</th>
                      <th className="py-2.5 px-3 font-medium">匹配状态 / 警示</th>
                      <th className="py-2.5 px-3 font-medium text-right">导入操作</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-neutral-850/60">
                    {filteredCandidates.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="py-8 text-center text-neutral-500">
                          没有符合当前筛选条件的游戏
                        </td>
                      </tr>
                    ) : (
                      filteredCandidates.map((g) => {
                        const curAction = decisions[g.external_id] || { type: 'Skip' };
                        return (
                          <tr key={g.external_id} className="hover:bg-neutral-900/40 transition-colors">
                            <td className="py-2.5 px-3">
                              <div className="font-semibold text-neutral-200">{g.title_name}</div>
                              <div className="font-mono text-[10px] text-neutral-500">{g.external_id}</div>
                            </td>
                            <td className="py-2.5 px-3 whitespace-nowrap">
                              <span className="font-mono font-bold text-emerald-400">
                                {g.playtime_hours.toFixed(1)} h
                              </span>
                            </td>
                            <td className="py-2.5 px-3 whitespace-nowrap font-mono text-neutral-400">
                              {g.launches != null ? `${g.launches} 次` : '—'}
                            </td>
                            <td className="py-2.5 px-3">
                              <div className="space-y-1">
                                {g.match_status === 'ExactMapped' && (
                                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
                                    <CheckCircle2 className="w-3 h-3" />
                                    <span>已绑定: {g.matched_work_title || '作品'}</span>
                                  </span>
                                )}
                                {g.match_status === 'SuggestedMatch' && (
                                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] bg-sky-500/10 text-sky-400 border border-sky-500/20 font-medium">
                                    <Sparkles className="w-3 h-3" />
                                    <span>建议关联: {g.matched_work_title}</span>
                                  </span>
                                )}
                                {g.match_status === 'NewGame' && (
                                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] bg-purple-500/10 text-purple-400 border border-purple-500/20 font-medium">
                                    <Plus className="w-3 h-3" />
                                    <span>新作品</span>
                                  </span>
                                )}

                                {g.warning && (
                                  <div className="text-[10px] text-amber-400 flex items-center gap-1">
                                    <AlertTriangle className="w-3 h-3 shrink-0" />
                                    <span>{g.warning}</span>
                                  </div>
                                )}
                              </div>
                            </td>
                            <td className="py-2.5 px-3 text-right whitespace-nowrap">
                              <div className="inline-flex items-center gap-1.5 justify-end">
                                <select
                                  value={curAction.type}
                                  onChange={(e) => {
                                    const val = e.target.value;
                                    if (val === 'CreateNewWork') {
                                      handleSetAction(g.external_id, {
                                        type: 'CreateNewWork',
                                        title: g.title_name,
                                      });
                                    } else if (val === 'LinkExistingWork') {
                                      const defaultWorkId = g.matched_work_id || (vaultGames[0]?.id ?? '');
                                      handleSetAction(g.external_id, {
                                        type: 'LinkExistingWork',
                                        work_id: defaultWorkId,
                                      });
                                    } else {
                                      handleSetAction(g.external_id, { type: 'Skip' });
                                    }
                                  }}
                                  className="px-2 py-1 bg-neutral-900 border border-neutral-750 rounded text-xs text-neutral-200 focus:outline-none focus:border-red-500"
                                >
                                  <option value="CreateNewWork">新建作品</option>
                                  <option value="LinkExistingWork">关联已有</option>
                                  <option value="Skip">跳过</option>
                                </select>

                                {curAction.type === 'LinkExistingWork' && (
                                  <select
                                    value={(curAction as any).work_id}
                                    onChange={(e) => {
                                      handleSetAction(g.external_id, {
                                        type: 'LinkExistingWork',
                                        work_id: e.target.value,
                                      });
                                    }}
                                    className="px-2 py-1 bg-neutral-900 border border-neutral-750 rounded text-xs text-neutral-200 max-w-[130px] truncate focus:outline-none"
                                  >
                                    {vaultGames.map((w) => (
                                      <option key={w.id} value={w.id}>
                                        {w.title}
                                      </option>
                                    ))}
                                  </select>
                                )}
                              </div>
                            </td>
                          </tr>
                        );
                      })
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Error Message */}
          {errorMsg && (
            <div className="p-3.5 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-300 text-xs flex items-start gap-2">
              <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0 mt-0.5" />
              <span>{errorMsg}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-neutral-800 bg-neutral-950/80 flex items-center justify-between shrink-0 text-xs">
          <div>
            {preview && (
              <span className="text-neutral-400">
                准备导入: <strong className="text-purple-400">{decisionCounts.toCreate}</strong> 款新建，
                <strong className="text-sky-400">{decisionCounts.toLink}</strong> 款关联，
                <strong className="text-neutral-500">{decisionCounts.toSkip}</strong> 款跳过
              </span>
            )}
          </div>

          <div className="flex items-center gap-2.5">
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-xl bg-neutral-800 hover:bg-neutral-750 text-neutral-300 font-medium transition-all"
            >
              {outcome ? '完成' : '取消'}
            </button>

            {preview && !outcome && (
              <button
                onClick={handleSubmit}
                disabled={submitting || (decisionCounts.toCreate === 0 && decisionCounts.toLink === 0)}
                className="px-5 py-2 rounded-xl bg-red-600 hover:bg-red-500 text-white font-bold transition-all shadow-lg shadow-red-600/20 flex items-center gap-2 disabled:opacity-50 cursor-pointer"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${submitting ? 'animate-spin' : ''}`} />
                <span>{submitting ? '写入事务中...' : '确认同步入库'}</span>
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

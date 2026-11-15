import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  X,
  Search,
  Sparkles,
  Gamepad2,
  Check,
  Loader2,
  AlertCircle,
} from 'lucide-react';
import { CoverCandidate } from '../types';

interface OnlineCoverModalProps {
  workId: string;
  workTitle: string;
  onClose: () => void;
  onCoverApplied: () => void;
}

export const OnlineCoverModal: React.FC<OnlineCoverModalProps> = ({
  workId,
  workTitle,
  onClose,
  onCoverApplied,
}) => {
  const [searchTerm, setSearchTerm] = useState(workTitle);
  const [candidates, setCandidates] = useState<CoverCandidate[]>([]);
  const [loading, setLoading] = useState(false);
  const [applyingUrl, setApplyingUrl] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err: any) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      setErrorMsg(err?.message || String(err));
      return null;
    }
  };

  const handleSearch = useCallback(
    async (query?: string) => {
      setLoading(true);
      setErrorMsg(null);
      try {
        const res = await safeInvoke<CoverCandidate[]>('fetch_cover_candidates', {
          workId,
          searchTerm: query !== undefined ? query : searchTerm,
        });
        setCandidates(res || []);
      } finally {
        setLoading(false);
      }
    },
    [workId, searchTerm]
  );

  // Initial candidate lookup on mount
  useEffect(() => {
    handleSearch('');
  }, []);

  const handleApplyCover = async (candidate: CoverCandidate) => {
    setApplyingUrl(candidate.image_url);
    setErrorMsg(null);
    try {
      const res = await safeInvoke('download_and_set_work_cover', {
        workId,
        imageUrl: candidate.image_url,
        userSelected: true,
      });
      if (res) {
        onCoverApplied();
        onClose();
      }
    } catch (err: any) {
      setErrorMsg(err?.message || String(err));
    } finally {
      setApplyingUrl(null);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-in fade-in">
      <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-4xl max-h-[88vh] flex flex-col shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="p-5 border-b border-neutral-800 flex items-center justify-between bg-neutral-900/80">
          <div className="flex items-center gap-3 min-w-0">
            <span className="p-2 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
              <Sparkles className="w-5 h-5" />
            </span>
            <div className="min-w-0">
              <h3 className="text-base font-bold text-white truncate">
                在线匹配官方封面 — 《{workTitle}》
              </h3>
              <p className="text-xs text-neutral-400 mt-0.5">
                实时检索 Nintendo Switch (Nlib) 与 Steam 官方海报素材库
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Search Bar */}
        <div className="p-4 border-b border-neutral-800/80 bg-neutral-950/50">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleSearch();
            }}
            className="flex gap-2"
          >
            <div className="relative flex-1">
              <Search className="w-4 h-4 text-neutral-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
              <input
                type="text"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                placeholder="输入游戏名称、Switch 16位 Title ID 或 Steam AppID..."
                className="w-full pl-10 pr-4 py-2 bg-neutral-900 border border-neutral-700/80 rounded-xl text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 transition-colors"
              />
            </div>
            <button
              type="submit"
              disabled={loading}
              className="px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold flex items-center gap-1.5 transition-colors disabled:opacity-50 cursor-pointer"
            >
              {loading ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Search className="w-3.5 h-3.5" />}
              <span>检索素材</span>
            </button>
          </form>
          <div className="text-[11px] text-neutral-500 mt-2 flex items-center gap-2 flex-wrap">
            <span>支持:</span>
            <span className="font-mono bg-neutral-800/80 px-1.5 py-0.5 rounded text-neutral-400">
              Steam 游戏名 / AppID
            </span>
            <span className="font-mono bg-neutral-800/80 px-1.5 py-0.5 rounded text-neutral-400">
              Switch 16位 Title ID (如 01007EF00011E000)
            </span>
          </div>
        </div>

        {/* Error Alert */}
        {errorMsg && (
          <div className="mx-6 mt-4 p-3 rounded-xl bg-red-950/40 border border-red-800/80 text-xs text-red-300 flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{errorMsg}</span>
          </div>
        )}

        {/* Candidates Grid */}
        <div className="flex-1 overflow-y-auto p-6">
          {loading ? (
            <div className="flex flex-col items-center justify-center py-20 text-center">
              <Loader2 className="w-8 h-8 text-emerald-400 animate-spin mb-3" />
              <p className="text-xs text-neutral-400">正在检索官方图源素材库并验证分辨率...</p>
            </div>
          ) : candidates.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-20 text-center">
              <Gamepad2 className="w-12 h-12 text-neutral-700 mb-3" />
              <p className="text-xs font-semibold text-neutral-300 mb-1">未找到匹配的在线封面</p>
              <p className="text-xs text-neutral-500 max-w-sm leading-relaxed">
                你可以尝试精简游戏名称（如去除非官方前缀、版本号），或直接在搜索框输入 Steam AppID 或 Switch Title ID。
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-5">
              {candidates.map((cand, idx) => {
                const isApplying = applyingUrl === cand.image_url;
                return (
                  <div
                    key={`${cand.image_url}-${idx}`}
                    className="flex flex-col justify-between rounded-xl border border-neutral-800 bg-neutral-950 p-3 hover:border-neutral-700 transition-all group"
                  >
                    <div>
                      {/* Image Preview Box */}
                      <div className="w-full h-48 rounded-lg overflow-hidden relative bg-neutral-900 border border-neutral-850 flex items-center justify-center">
                        {/* Ambient Backdrop */}
                        <div
                          className="absolute inset-0 bg-cover bg-center blur-xl opacity-35 scale-110 pointer-events-none"
                          style={{ backgroundImage: `url(${cand.image_url})` }}
                        />
                        <img
                          src={cand.image_url}
                          alt={cand.title}
                          className="w-full h-full object-contain relative z-10 transition-transform group-hover:scale-[1.02]"
                          loading="lazy"
                        />
                        {/* Provider Badge */}
                        <div className="absolute top-2 left-2 z-20 flex gap-1">
                          <span
                            className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider backdrop-blur-md shadow-md ${
                              cand.provider === 'nlib'
                                ? 'bg-red-500/80 text-white'
                                : 'bg-blue-600/80 text-white'
                            }`}
                          >
                            {cand.provider === 'nlib' ? 'Switch' : 'Steam'}
                          </span>
                          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-black/60 text-neutral-300 backdrop-blur-md">
                            {cand.aspect_ratio === '2:3'
                              ? '2:3 竖版'
                              : cand.aspect_ratio === '1:1'
                              ? '1:1 方形'
                              : '横幅'}
                          </span>
                        </div>
                      </div>

                      {/* Title & Description */}
                      <div className="mt-3">
                        <h4 className="text-xs font-bold text-white line-clamp-1 group-hover:text-emerald-300 transition-colors">
                          {cand.title}
                        </h4>
                        {cand.description && (
                          <p className="text-[11px] text-neutral-400 mt-1 line-clamp-2 font-mono">
                            {cand.description}
                          </p>
                        )}
                      </div>
                    </div>

                    {/* Action Button */}
                    <div className="mt-4 pt-3 border-t border-neutral-850">
                      <button
                        onClick={() => handleApplyCover(cand)}
                        disabled={isApplying || applyingUrl !== null}
                        className="w-full py-2 px-3 rounded-lg bg-emerald-600/20 hover:bg-emerald-600 text-emerald-300 hover:text-white border border-emerald-500/30 text-xs font-semibold flex items-center justify-center gap-1.5 transition-all disabled:opacity-50 cursor-pointer"
                      >
                        {isApplying ? (
                          <>
                            <Loader2 className="w-3.5 h-3.5 animate-spin" />
                            <span>正在下载并持久化...</span>
                          </>
                        ) : (
                          <>
                            <Check className="w-3.5 h-3.5" />
                            <span>应用此封面</span>
                          </>
                        )}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

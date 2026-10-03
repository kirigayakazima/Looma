import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Sparkles,
  Link,
  Boxes,
  Tag,
  Check,
  RefreshCw,
  FolderPlus,
  CheckCheck,
  TrendingUp,
} from 'lucide-react';
import { SmartInsightsReport, Suggestion, SuggestionType } from '../types';
import { useI18n } from '../i18n';

interface InsightsViewProps {
  onRefreshAll?: () => Promise<void>;
}

export const InsightsView: React.FC<InsightsViewProps> = ({ onRefreshAll }) => {
  const { t } = useI18n();
  const [report, setReport] = useState<SmartInsightsReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [filterType, setFilterType] = useState<'all' | SuggestionType>('all');
  const [applyingId, setApplyingId] = useState<string | null>(null);
  const [batchApplying, setBatchApplying] = useState(false);
  const [feedbackMsg, setFeedbackMsg] = useState<string | null>(null);

  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  const loadInsights = useCallback(async () => {
    setLoading(true);
    try {
      const data = await safeInvoke<SmartInsightsReport>('get_smart_insights');
      if (data) {
        setReport(data);
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadInsights();
  }, [loadInsights]);

  const handleApply = async (s: Suggestion) => {
    setApplyingId(s.id);
    try {
      const ok = await safeInvoke<boolean>('apply_smart_suggestion', { suggestion: s });
      if (ok) {
        setFeedbackMsg(t.insights.applied);
        setTimeout(() => setFeedbackMsg(null), 3000);
        await loadInsights();
        if (onRefreshAll) await onRefreshAll();
      }
    } finally {
      setApplyingId(null);
    }
  };

  const handleBatchApplyHigh = async () => {
    if (!report) return;
    setBatchApplying(true);
    try {
      const highConfidence = [
        ...report.relation_suggestions,
        ...report.cluster_suggestions,
      ].filter((s) => s.confidence >= 0.85);

      for (const s of highConfidence) {
        await safeInvoke<boolean>('apply_smart_suggestion', { suggestion: s });
      }

      setFeedbackMsg(t.insights.autoApplySuccess);
      setTimeout(() => setFeedbackMsg(null), 3000);
      await loadInsights();
      if (onRefreshAll) await onRefreshAll();
    } finally {
      setBatchApplying(false);
    }
  };

  const allSuggestions = report
    ? [
        ...report.relation_suggestions,
        ...report.cluster_suggestions,
        ...report.tag_suggestions,
      ]
    : [];

  const filteredSuggestions = allSuggestions.filter((s) => {
    if (filterType === 'all') return true;
    return s.suggestion_type === filterType;
  });

  return (
    <div className="space-y-6 max-w-6xl pb-12">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <div className="flex items-center space-x-2">
            <Sparkles className="w-5 h-5 text-amber-500" />
            <h3 className="text-base font-semibold text-neutral-100">{t.insights.title}</h3>
            <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-amber-500/10 text-amber-500 border border-amber-500/20">
              Phase 6
            </span>
          </div>
          <p className="text-xs text-neutral-400 mt-1">{t.insights.subtitle}</p>
        </div>

        <div className="flex items-center space-x-2">
          {allSuggestions.some((s) => s.confidence >= 0.85 && s.suggestion_type !== 'tag') && (
            <button
              onClick={handleBatchApplyHigh}
              disabled={batchApplying || loading}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-amber-500 hover:bg-amber-600 disabled:opacity-50 text-white text-xs font-medium shadow-sm transition-colors cursor-pointer"
            >
              <CheckCheck className="w-3.5 h-3.5" />
              <span>{batchApplying ? t.insights.applying : t.insights.autoApplyHigh}</span>
            </button>
          )}

          <button
            onClick={loadInsights}
            disabled={loading}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-neutral-850 hover:bg-neutral-800 border border-neutral-700/60 text-neutral-300 text-xs font-medium transition-colors cursor-pointer"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
            <span>{loading ? t.insights.refreshing : t.insights.refreshBtn}</span>
          </button>
        </div>
      </div>

      {feedbackMsg && (
        <div className="p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs flex items-center space-x-2">
          <Check className="w-4 h-4" />
          <span>{feedbackMsg}</span>
        </div>
      )}

      {/* Summary KPI Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-xs text-neutral-400">{t.insights.tabRelations}</span>
            <div className="text-xl font-bold text-neutral-100">
              {report?.relation_suggestions.length || 0}
            </div>
          </div>
          <div className="w-9 h-9 rounded-lg bg-indigo-500/10 text-indigo-400 flex items-center justify-center">
            <Link className="w-4 h-4" />
          </div>
        </div>

        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-xs text-neutral-400">{t.insights.tabClusters}</span>
            <div className="text-xl font-bold text-neutral-100">
              {report?.cluster_suggestions.length || 0}
            </div>
          </div>
          <div className="w-9 h-9 rounded-lg bg-emerald-500/10 text-emerald-400 flex items-center justify-center">
            <Boxes className="w-4 h-4" />
          </div>
        </div>

        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-xs text-neutral-400">{t.insights.tabTags}</span>
            <div className="text-xl font-bold text-neutral-100">
              {report?.tag_suggestions.length || 0}
            </div>
          </div>
          <div className="w-9 h-9 rounded-lg bg-amber-500/10 text-amber-500 flex items-center justify-center">
            <Tag className="w-4 h-4" />
          </div>
        </div>
      </div>

      {/* Filter Tabs */}
      <div className="flex items-center space-x-2 border-b border-neutral-800 pb-2">
        <button
          onClick={() => setFilterType('all')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
            filterType === 'all'
              ? 'bg-neutral-800 text-neutral-100 shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200'
          }`}
        >
          {t.insights.tabAll} ({allSuggestions.length})
        </button>
        <button
          onClick={() => setFilterType('relation')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
            filterType === 'relation'
              ? 'bg-neutral-800 text-neutral-100 shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200'
          }`}
        >
          {t.insights.tabRelations} ({report?.relation_suggestions.length || 0})
        </button>
        <button
          onClick={() => setFilterType('cluster')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
            filterType === 'cluster'
              ? 'bg-neutral-800 text-neutral-100 shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200'
          }`}
        >
          {t.insights.tabClusters} ({report?.cluster_suggestions.length || 0})
        </button>
        <button
          onClick={() => setFilterType('tag')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
            filterType === 'tag'
              ? 'bg-neutral-800 text-neutral-100 shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200'
          }`}
        >
          {t.insights.tabTags} ({report?.tag_suggestions.length || 0})
        </button>
      </div>

      {/* Suggestions List */}
      {filteredSuggestions.length === 0 ? (
        <div className="p-12 text-center rounded-xl bg-neutral-900 border border-neutral-800 space-y-3">
          <div className="w-12 h-12 rounded-full bg-neutral-850 flex items-center justify-center mx-auto text-neutral-400">
            <Sparkles className="w-6 h-6 text-amber-500/70" />
          </div>
          <h4 className="text-sm font-semibold text-neutral-200">{t.insights.emptyTitle}</h4>
          <p className="text-xs text-neutral-400 max-w-md mx-auto leading-relaxed">
            {t.insights.emptyDesc}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {filteredSuggestions.map((s) => {
            const isRelation = s.suggestion_type === 'relation';
            const isCluster = s.suggestion_type === 'cluster';
            const isApplying = applyingId === s.id;

            return (
              <div
                key={s.id}
                className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 hover:border-neutral-700 transition-all flex flex-col justify-between space-y-3"
              >
                <div className="space-y-2">
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center space-x-2">
                      <span
                        className={`p-1.5 rounded-md ${
                          isRelation
                            ? 'bg-indigo-500/10 text-indigo-400'
                            : isCluster
                            ? 'bg-emerald-500/10 text-emerald-400'
                            : 'bg-amber-500/10 text-amber-500'
                        }`}
                      >
                        {isRelation ? (
                          <Link className="w-3.5 h-3.5" />
                        ) : isCluster ? (
                          <FolderPlus className="w-3.5 h-3.5" />
                        ) : (
                          <Tag className="w-3.5 h-3.5" />
                        )}
                      </span>
                      <span className="text-[11px] font-medium text-neutral-400 uppercase tracking-wider">
                        {isRelation
                          ? t.insights.tabRelations
                          : isCluster
                          ? t.insights.tabClusters
                          : t.insights.tabTags}
                      </span>
                    </div>

                    <div className="flex items-center space-x-1 text-[11px] font-mono text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
                      <TrendingUp className="w-3 h-3" />
                      <span>{Math.round(s.confidence * 100)}%</span>
                    </div>
                  </div>

                  <h5 className="text-xs font-semibold text-neutral-100 leading-snug">
                    {s.title}
                  </h5>

                  <p className="text-xs text-neutral-400 leading-relaxed">
                    {s.description}
                  </p>

                  {s.tags.length > 0 && (
                    <div className="flex flex-wrap gap-1.5 pt-1">
                      {s.tags.map((tag, idx) => (
                        <span
                          key={idx}
                          className="px-2 py-0.5 rounded text-[10px] bg-neutral-850 text-neutral-300 border border-neutral-700/50"
                        >
                          #{tag}
                        </span>
                      ))}
                    </div>
                  )}
                </div>

                <div className="pt-2 border-t border-neutral-800/80 flex items-center justify-between">
                  <span className="text-[11px] text-neutral-500 truncate max-w-[200px]">
                    {s.source_name}
                  </span>

                  {s.suggestion_type !== 'tag' && (
                    <button
                      onClick={() => handleApply(s)}
                      disabled={isApplying}
                      className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white text-xs font-medium transition-colors cursor-pointer shadow-sm"
                    >
                      <Check className="w-3 h-3" />
                      <span>{isApplying ? t.insights.applying : t.insights.applyBtn}</span>
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

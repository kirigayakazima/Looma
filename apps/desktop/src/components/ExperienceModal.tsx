import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Compass, X, Calendar, Monitor, FileText, Check } from 'lucide-react';

interface ExperienceModalProps {
  isOpen: boolean;
  onClose: () => void;
  workId: string;
  workTitle: string;
  defaultEventType?: string;
  defaultPlatform?: string;
  defaultNote?: string;
  onSuccess?: () => Promise<void> | void;
}

export const ExperienceModal: React.FC<ExperienceModalProps> = ({
  isOpen,
  onClose,
  workId,
  workTitle,
  defaultEventType = 'completed',
  defaultPlatform = '',
  defaultNote = '',
  onSuccess,
}) => {
  const [eventType, setEventType] = useState(defaultEventType);
  const [title, setTitle] = useState('');
  const [precision, setPrecision] = useState<'unknown' | 'year' | 'month' | 'day'>('unknown');
  const [dateValue, setDateValue] = useState('');
  const [platform, setPlatform] = useState(defaultPlatform);
  const [note, setNote] = useState(defaultNote);
  const [submitting, setSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Initialize or reset when opened
  useEffect(() => {
    if (isOpen) {
      setEventType(defaultEventType);
      setTitle(
        defaultEventType === 'completed'
          ? '通关达成'
          : defaultEventType === 'replayed'
          ? '二周目通关'
          : defaultEventType === 'started'
          ? '入坑开启'
          : ''
      );
      // Default to unknown time to prevent artificial distortion
      setPrecision('unknown');
      setDateValue('');
      setPlatform(defaultPlatform);
      setNote(defaultNote);
      setErrorMsg(null);
    }
  }, [isOpen, defaultEventType, defaultPlatform, defaultNote]);

  if (!isOpen) return null;

  const quickPlatforms = ['PC', 'PS5', 'PS4', 'Switch', 'Xbox', 'Steam Deck', 'iOS', 'Android'];

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    setErrorMsg(null);

    try {
      const isCompletion = eventType === 'completed';
      const occurredPrecision = precision;
      const occurredValue = precision === 'unknown' ? null : dateValue.trim() || null;
      const contextJson = platform.trim() ? { platform: platform.trim() } : {};

      if (isCompletion) {
        await invoke('complete_work_and_record_experience', {
          workId,
          experienceTitle: title.trim() || undefined,
          occurredPrecision,
          occurredValue,
          note: note.trim() || undefined,
          contextJson,
        });
      } else {
        await invoke('record_experience', {
          subjectId: workId,
          eventType,
          title: title.trim() || undefined,
          note: note.trim() || undefined,
          occurredPrecision,
          occurredValue,
          contextJson,
        });
      }

      if (onSuccess) await onSuccess();
      onClose();
    } catch (err: any) {
      setErrorMsg(err?.message || String(err));
    } finally {
      setSubmitting(false);
    }
  };

  const handleJustCompleteWithoutFootprint = async () => {
    setSubmitting(true);
    setErrorMsg(null);
    try {
      await invoke('update_work_status', { id: workId, status: 'completed' });
      if (onSuccess) await onSuccess();
      onClose();
    } catch (err: any) {
      setErrorMsg(err?.message || String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in">
      <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl flex flex-col max-h-[90vh]">
        {/* Modal Header */}
        <div className="p-5 border-b border-neutral-800 flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400">
              <Compass className="w-4 h-4" />
            </div>
            <div>
              <h3 className="font-bold text-sm text-white">记录个人经历与足迹</h3>
              <p className="text-xs text-neutral-400 mt-0.5 truncate max-w-xs">
                作品：《{workTitle}》
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Modal Body Form */}
        <form onSubmit={handleSubmit} className="p-5 space-y-4 overflow-y-auto flex-1">
          {errorMsg && (
            <div className="p-3 rounded-lg bg-red-950/60 border border-red-800 text-red-300 text-xs leading-relaxed">
              {errorMsg}
            </div>
          )}

          {/* Event Type & Footprint Title */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                经历事件类型 <span className="text-emerald-400">*</span>
              </label>
              <select
                value={eventType}
                onChange={(e) => {
                  const val = e.target.value;
                  setEventType(val);
                  if (val === 'completed' && !title) setTitle('通关达成');
                  else if (val === 'replayed' && !title) setTitle('二周目通关');
                  else if (val === 'started' && !title) setTitle('入坑开启');
                }}
                className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 focus:outline-none focus:border-emerald-500"
              >
                <option value="completed">通关达成 (Completed)</option>
                <option value="replayed">多周目重温 (Replayed)</option>
                <option value="started">入坑开启 (Started)</option>
                <option value="progress_milestone">关键里程碑 (Milestone)</option>
                <option value="reflection">心得感悟 (Reflection)</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-medium text-neutral-300 mb-1.5">
                足迹标题 (可选)
              </label>
              <input
                type="text"
                placeholder="例如: 全成就达成 / 击败隐藏 Boss"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500"
              />
            </div>
          </div>

          {/* Date & Precision (Default unknown, never forced today) */}
          <div className="space-y-2 p-3.5 rounded-xl bg-neutral-950 border border-neutral-850">
            <label className="block text-xs font-medium text-neutral-300 flex items-center justify-between">
              <span className="flex items-center gap-1.5">
                <Calendar className="w-3.5 h-3.5 text-neutral-400" />
                <span>发生时间与时间精度</span>
              </span>
              <span className="text-[10px] text-neutral-500 font-normal">
                支持年份、年月、具体日期或未知
              </span>
            </label>

            {/* Precision Tabs */}
            <div className="flex gap-1.5 p-1 rounded-lg bg-neutral-900 border border-neutral-800 text-xs">
              <button
                type="button"
                onClick={() => {
                  setPrecision('unknown');
                  setDateValue('');
                }}
                className={`flex-1 py-1 rounded text-center transition-colors ${
                  precision === 'unknown'
                    ? 'bg-neutral-800 text-emerald-400 font-medium shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                未知发生时间
              </button>
              <button
                type="button"
                onClick={() => {
                  setPrecision('year');
                  if (!dateValue || dateValue.length > 4) {
                    setDateValue(new Date().getFullYear().toString());
                  }
                }}
                className={`flex-1 py-1 rounded text-center transition-colors ${
                  precision === 'year'
                    ? 'bg-neutral-800 text-emerald-400 font-medium shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                仅年份
              </button>
              <button
                type="button"
                onClick={() => {
                  setPrecision('month');
                  if (!dateValue || dateValue.length !== 7) {
                    const now = new Date();
                    setDateValue(`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`);
                  }
                }}
                className={`flex-1 py-1 rounded text-center transition-colors ${
                  precision === 'month'
                    ? 'bg-neutral-800 text-emerald-400 font-medium shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                年月
              </button>
              <button
                type="button"
                onClick={() => {
                  setPrecision('day');
                  if (!dateValue || dateValue.length !== 10) {
                    setDateValue(new Date().toISOString().substring(0, 10));
                  }
                }}
                className={`flex-1 py-1 rounded text-center transition-colors ${
                  precision === 'day'
                    ? 'bg-neutral-800 text-emerald-400 font-medium shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                具体日期
              </button>
            </div>

            {/* Dynamic Date Input */}
            {precision === 'unknown' ? (
              <p className="text-[11px] text-neutral-500 italic pt-1">
                已选择“未知发生时间”。系统将独立归档此足迹，绝不伪造或默认伪装为录入当天的日期。
              </p>
            ) : precision === 'year' ? (
              <input
                type="number"
                min="1980"
                max="2099"
                placeholder="例如: 2021"
                value={dateValue}
                onChange={(e) => setDateValue(e.target.value)}
                className="w-full px-3 py-1.5 bg-neutral-900 border border-neutral-800 rounded-lg text-xs text-neutral-100 font-mono focus:outline-none focus:border-emerald-500"
              />
            ) : precision === 'month' ? (
              <input
                type="month"
                value={dateValue}
                onChange={(e) => setDateValue(e.target.value)}
                className="w-full px-3 py-1.5 bg-neutral-900 border border-neutral-800 rounded-lg text-xs text-neutral-100 font-mono focus:outline-none focus:border-emerald-500"
              />
            ) : (
              <input
                type="date"
                value={dateValue}
                onChange={(e) => setDateValue(e.target.value)}
                className="w-full px-3 py-1.5 bg-neutral-900 border border-neutral-800 rounded-lg text-xs text-neutral-100 font-mono focus:outline-none focus:border-emerald-500"
              />
            )}
          </div>

          {/* Platform / Environment (Default empty, optional) */}
          <div>
            <label className="block text-xs font-medium text-neutral-300 mb-1.5 flex items-center gap-1.5">
              <Monitor className="w-3.5 h-3.5 text-neutral-400" />
              <span>游玩设备 / 平台 (可选)</span>
            </label>
            <input
              type="text"
              placeholder="例如: PC, PS5, Switch, 影院..."
              value={platform}
              onChange={(e) => setPlatform(e.target.value)}
              className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 mb-1.5"
            />
            <div className="flex flex-wrap gap-1">
              {quickPlatforms.map((p) => (
                <button
                  type="button"
                  key={p}
                  onClick={() => setPlatform(p)}
                  className={`text-[10px] px-2 py-0.5 rounded font-mono transition-colors ${
                    platform === p
                      ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                      : 'bg-neutral-950 text-neutral-500 hover:text-neutral-300 border border-neutral-850'
                  }`}
                >
                  {p}
                </button>
              ))}
            </div>
          </div>

          {/* Footprint Note */}
          <div>
            <label className="block text-xs font-medium text-neutral-300 mb-1.5 flex items-center gap-1.5">
              <FileText className="w-3.5 h-3.5 text-neutral-400" />
              <span>心得笔记与专属回忆 (可选)</span>
            </label>
            <textarea
              rows={3}
              placeholder="记录当时的通关感想、难度体验或专属记忆..."
              value={note}
              onChange={(e) => setNote(e.target.value)}
              className="w-full px-3 py-2 bg-neutral-950 border border-neutral-800 rounded-lg text-xs text-neutral-100 placeholder-neutral-500 focus:outline-none focus:border-emerald-500 resize-none leading-relaxed"
            />
          </div>

          {/* Footer Actions */}
          <div className="pt-2 flex items-center justify-between border-t border-neutral-800/80">
            {eventType === 'completed' ? (
              <button
                type="button"
                onClick={handleJustCompleteWithoutFootprint}
                disabled={submitting}
                className="text-xs text-neutral-400 hover:text-neutral-200 underline cursor-pointer disabled:opacity-50"
              >
                仅标记为已完成（不记足迹）
              </button>
            ) : (
              <div />
            )}

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium transition-colors"
              >
                取消
              </button>
              <button
                type="submit"
                disabled={submitting}
                className="px-5 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-neutral-950 text-xs font-bold transition-all disabled:opacity-50 flex items-center gap-1.5 shadow-md shadow-emerald-500/20"
              >
                <Check className="w-3.5 h-3.5" />
                <span>
                  {submitting
                    ? '保存中...'
                    : eventType === 'completed'
                    ? '保存足迹并通关'
                    : '保存足迹'}
                </span>
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};

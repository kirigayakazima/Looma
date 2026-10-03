import React, { useState } from 'react';
import { Memory } from '../types';
import { BookOpen, Plus, Calendar } from 'lucide-react';
import { useI18n } from '../i18n';

interface MemoriesViewProps {
  memories: Memory[];
  onCreateMemory: (mem: { title: string; content: string; category?: string }) => Promise<void>;
}

export const MemoriesView: React.FC<MemoriesViewProps> = ({ memories, onCreateMemory }) => {
  const { t } = useI18n();
  const [showModal, setShowModal] = useState(false);
  const [title, setTitle] = useState('');
  const [category, setCategory] = useState('journal');
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !content.trim()) return;
    setLoading(true);
    try {
      await onCreateMemory({
        title: title.trim(),
        content: content.trim(),
        category: category.trim() || undefined,
      });
      setTitle('');
      setContent('');
      setShowModal(false);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-4 max-w-5xl">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-neutral-100">{t.memories.title}</h3>
          <p className="text-xs text-neutral-400">{t.memories.subtitle}</p>
        </div>
        <button
          onClick={() => setShowModal(true)}
          className="flex items-center space-x-1.5 text-xs px-3 py-1.5 rounded-md bg-amber-600 hover:bg-amber-500 text-white font-medium transition-colors shadow-sm shadow-amber-600/20"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{t.memories.writeBtn}</span>
        </button>
      </div>

      {showModal && (
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-700/80 shadow-lg space-y-3">
          <h4 className="text-sm font-semibold text-neutral-200">{t.memories.modalTitle}</h4>
          <form onSubmit={handleSubmit} className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
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
            </div>
            <div>
              <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.memories.contentLabel}</label>
              <textarea
                rows={4}
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
                onClick={() => setShowModal(false)}
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

      {memories.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <BookOpen className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.memories.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.memories.emptyDesc}
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {memories.map((mem) => (
            <div
              key={mem.id}
              className="p-4 rounded-lg bg-neutral-900 border border-neutral-800 hover:border-neutral-700 transition-colors"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center space-x-2">
                  <span className="text-xs font-semibold text-neutral-200">{mem.title}</span>
                  {mem.category && (
                    <span className="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/10 text-amber-400 font-mono">
                      {mem.category}
                    </span>
                  )}
                </div>
                <div className="flex items-center space-x-1 text-[11px] text-neutral-500">
                  <Calendar className="w-3 h-3" />
                  <span>{new Date(mem.recorded_at).toLocaleDateString()}</span>
                </div>
              </div>
              <p className="text-xs text-neutral-400 whitespace-pre-wrap font-sans">{mem.content}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

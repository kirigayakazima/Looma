import React, { useState } from 'react';
import { Entity } from '../types';
import { Boxes, Plus, Tag } from 'lucide-react';
import { useI18n } from '../i18n';

interface EntitiesViewProps {
  entities: Entity[];
  onCreateEntity: (entity: { title: string; entity_type: string; description?: string }) => Promise<void>;
}

export const EntitiesView: React.FC<EntitiesViewProps> = ({ entities, onCreateEntity }) => {
  const { t } = useI18n();
  const [showModal, setShowModal] = useState(false);
  const [title, setTitle] = useState('');
  const [entityType, setEntityType] = useState('project');
  const [description, setDescription] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setLoading(true);
    try {
      await onCreateEntity({
        title: title.trim(),
        entity_type: entityType.trim(),
        description: description.trim() || undefined,
      });
      setTitle('');
      setDescription('');
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
          <h3 className="text-base font-semibold text-neutral-100">{t.entities.title}</h3>
          <p className="text-xs text-neutral-400">{t.entities.subtitle}</p>
        </div>
        <button
          onClick={() => setShowModal(true)}
          className="flex items-center space-x-1.5 text-xs px-3 py-1.5 rounded-md bg-indigo-600 hover:bg-indigo-500 text-white font-medium transition-colors shadow-sm shadow-indigo-600/20"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{t.entities.newEntityBtn}</span>
        </button>
      </div>

      {showModal && (
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-700/80 shadow-lg space-y-3">
          <h4 className="text-sm font-semibold text-neutral-200">{t.entities.modalTitle}</h4>
          <form onSubmit={handleSubmit} className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.entities.titleLabel}</label>
                <input
                  type="text"
                  required
                  placeholder={t.entities.titlePlaceholder}
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500"
                />
              </div>
              <div>
                <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.entities.typeLabel}</label>
                <input
                  type="text"
                  required
                  placeholder={t.entities.typePlaceholder}
                  value={entityType}
                  onChange={(e) => setEntityType(e.target.value)}
                  className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500"
                />
              </div>
            </div>
            <div>
              <label className="block text-[11px] font-medium text-neutral-400 mb-1">{t.entities.descLabel}</label>
              <textarea
                rows={2}
                placeholder={t.entities.descPlaceholder}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="w-full bg-neutral-800 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-100 focus:outline-none focus:border-indigo-500 resize-none"
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
                className="text-xs px-3 py-1.5 rounded bg-indigo-600 hover:bg-indigo-500 text-white font-medium transition-colors disabled:opacity-50"
              >
                {loading ? t.common.creating : t.entities.newEntityBtn}
              </button>
            </div>
          </form>
        </div>
      )}

      {entities.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <Boxes className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.entities.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.entities.emptyDesc}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {entities.map((item) => (
            <div
              key={item.id}
              className="p-4 rounded-lg bg-neutral-900 border border-neutral-800 hover:border-neutral-700 transition-colors flex flex-col justify-between"
            >
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] bg-indigo-500/10 text-indigo-400 font-mono">
                    <Tag className="w-2.5 h-2.5 mr-0.5" />
                    {item.entity_type}
                  </span>
                  <span className="text-[10px] text-neutral-500">
                    {new Date(item.created_at).toLocaleDateString()}
                  </span>
                </div>
                <h4 className="text-sm font-medium text-neutral-100 mb-1">{item.title}</h4>
                {item.description && (
                  <p className="text-xs text-neutral-400 line-clamp-2">{item.description}</p>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

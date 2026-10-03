import React from 'react';
import { Collection } from '../types';
import { BookmarkCheck, FolderHeart } from 'lucide-react';
import { useI18n } from '../i18n';

interface CollectionsViewProps {
  collections: Collection[];
}

export const CollectionsView: React.FC<CollectionsViewProps> = ({ collections }) => {
  const { t } = useI18n();

  return (
    <div className="space-y-4 max-w-5xl">
      <div>
        <h3 className="text-base font-semibold text-neutral-100">{t.collections.title}</h3>
        <p className="text-xs text-neutral-400">{t.collections.subtitle}</p>
      </div>

      {collections.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <BookmarkCheck className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.collections.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.collections.emptyDesc}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {collections.map((col) => (
            <div
              key={col.id}
              className="p-4 rounded-lg bg-neutral-900 border border-neutral-800 hover:border-neutral-700 transition-colors"
            >
              <div className="flex items-center space-x-2 mb-1.5">
                <FolderHeart className="w-4 h-4 text-emerald-400" />
                <h4 className="text-sm font-medium text-neutral-200">{col.title}</h4>
              </div>
              {col.description && <p className="text-xs text-neutral-400">{col.description}</p>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

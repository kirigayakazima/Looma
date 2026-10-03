import React from 'react';
import { Asset } from '../types';
import { FolderArchive, FileText, Image, Video, Music, Archive } from 'lucide-react';
import { useI18n } from '../i18n';

interface AssetsViewProps {
  assets: Asset[];
  onRefresh: () => void;
}

export const AssetsView: React.FC<AssetsViewProps> = ({ assets, onRefresh }) => {
  const { t } = useI18n();

  const getKindIcon = (kind: string) => {
    switch (kind) {
      case 'image':
        return <Image className="w-4 h-4 text-blue-400" />;
      case 'video':
        return <Video className="w-4 h-4 text-purple-400" />;
      case 'audio':
        return <Music className="w-4 h-4 text-green-400" />;
      case 'archive':
        return <Archive className="w-4 h-4 text-amber-400" />;
      default:
        return <FileText className="w-4 h-4 text-neutral-400" />;
    }
  };

  const getKindLabel = (kind: string) => {
    switch (kind) {
      case 'image':
        return t.assets.kindImage;
      case 'video':
        return t.assets.kindVideo;
      case 'audio':
        return t.assets.kindAudio;
      case 'archive':
        return t.assets.kindArchive;
      case 'document':
        return t.assets.kindDocument;
      default:
        return kind;
    }
  };

  return (
    <div className="space-y-4 max-w-5xl">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-neutral-100">{t.assets.title}</h3>
          <p className="text-xs text-neutral-400">{t.assets.subtitle}</p>
        </div>
        <button
          onClick={onRefresh}
          className="text-xs px-3 py-1.5 rounded-md bg-neutral-800 hover:bg-neutral-700 text-neutral-200 transition-colors border border-neutral-700"
        >
          {t.common.refresh}
        </button>
      </div>

      {assets.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <FolderArchive className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.assets.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.assets.emptyDesc}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {assets.map((asset) => (
            <div
              key={asset.id}
              className="p-3 rounded-lg bg-neutral-900 border border-neutral-800 hover:border-neutral-700 transition-colors"
            >
              <div className="flex items-center space-x-2.5 mb-2">
                {getKindIcon(asset.kind)}
                <span className="text-xs font-medium text-neutral-200 truncate">
                  {asset.path?.split(/[\\/]/).pop() || asset.id}
                </span>
              </div>
              <div className="text-[11px] text-neutral-500 truncate">{asset.path}</div>
              <div className="mt-2 pt-2 border-t border-neutral-800/60 flex items-center justify-between text-[10px] text-neutral-500">
                <span className="uppercase">{getKindLabel(asset.kind)}</span>
                <span>{asset.size ? `${(asset.size / 1024).toFixed(0)} KB` : '—'}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

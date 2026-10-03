import React, { useState, useEffect, useMemo, useRef } from 'react';
import { Asset, Entity, Collection, Memory } from '../types';
import {
  Search,
  X,
  FileText,
  ImageIcon,
  Video,
  Music,
  Code,
  Archive,
  Box,
  FileQuestion,
  Layers,
  FolderHeart,
  BookOpen,
  ArrowRight,
  Sparkles,
} from 'lucide-react';
import { useI18n } from '../i18n';

interface GlobalSearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  assets: Asset[];
  entities: Entity[];
  collections: Collection[];
  memories: Memory[];
  onNavigate: (tab: 'home' | 'assets' | 'entities' | 'collections' | 'timeline' | 'memories' | 'settings', itemId?: string) => void;
}

type SearchCategory = 'all' | 'assets' | 'entities' | 'collections' | 'memories';

interface SearchResultItem {
  id: string;
  category: 'asset' | 'entity' | 'collection' | 'memory';
  title: string;
  subtitle: string;
  badge?: string;
  icon: React.ReactNode;
}

export const GlobalSearchModal: React.FC<GlobalSearchModalProps> = ({
  isOpen,
  onClose,
  assets,
  entities,
  collections,
  memories,
  onNavigate,
}) => {
  const { t } = useI18n();
  const [query, setQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<SearchCategory>('all');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input when modal opens
  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => {
        inputRef.current?.focus();
      }, 50);
    }
  }, [isOpen]);

  const getAssetIcon = (kind: string) => {
    const cls = 'w-4 h-4';
    switch (kind) {
      case 'image':
        return <ImageIcon className={`${cls} text-sky-400`} />;
      case 'video':
        return <Video className={`${cls} text-purple-400`} />;
      case 'audio':
        return <Music className={`${cls} text-emerald-400`} />;
      case 'document':
        return <FileText className={`${cls} text-blue-400`} />;
      case 'code':
        return <Code className={`${cls} text-amber-400`} />;
      case 'archive':
        return <Archive className={`${cls} text-orange-400`} />;
      case 'model3d':
        return <Box className={`${cls} text-rose-400`} />;
      default:
        return <FileQuestion className={`${cls} text-neutral-400`} />;
    }
  };

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    const items: SearchResultItem[] = [];

    // Search Assets
    if (activeCategory === 'all' || activeCategory === 'assets') {
      for (const a of assets) {
        const p = a.path || '';
        const filename = p.split(/[\\/]/).pop() || a.id;
        const matchesQuery =
          !q ||
          filename.toLowerCase().includes(q) ||
          p.toLowerCase().includes(q) ||
          a.kind.toLowerCase().includes(q) ||
          (a.mime_type && a.mime_type.toLowerCase().includes(q));

        if (matchesQuery) {
          items.push({
            id: a.id,
            category: 'asset',
            title: filename,
            subtitle: p || a.id,
            badge: a.kind,
            icon: getAssetIcon(a.kind),
          });
        }
      }
    }

    // Search Entities
    if (activeCategory === 'all' || activeCategory === 'entities') {
      for (const e of entities) {
        const propsStr = JSON.stringify(e.properties).toLowerCase();
        const matchesQuery =
          !q ||
          e.title.toLowerCase().includes(q) ||
          e.entity_type.toLowerCase().includes(q) ||
          (e.description && e.description.toLowerCase().includes(q)) ||
          propsStr.includes(q);

        if (matchesQuery) {
          items.push({
            id: e.id,
            category: 'entity',
            title: e.title,
            subtitle: e.description || `Type: ${e.entity_type}`,
            badge: e.entity_type,
            icon: <Layers className="w-4 h-4 text-indigo-400" />,
          });
        }
      }
    }

    // Search Collections
    if (activeCategory === 'all' || activeCategory === 'collections') {
      for (const c of collections) {
        const matchesQuery =
          !q ||
          c.title.toLowerCase().includes(q) ||
          (c.description && c.description.toLowerCase().includes(q));

        if (matchesQuery) {
          items.push({
            id: c.id,
            category: 'collection',
            title: c.title,
            subtitle: c.description || 'Collection',
            badge: 'collection',
            icon: <FolderHeart className="w-4 h-4 text-purple-400" />,
          });
        }
      }
    }

    // Search Memories
    if (activeCategory === 'all' || activeCategory === 'memories') {
      for (const m of memories) {
        const matchesQuery =
          !q ||
          m.title.toLowerCase().includes(q) ||
          m.content.toLowerCase().includes(q) ||
          (m.category && m.category.toLowerCase().includes(q));

        if (matchesQuery) {
          items.push({
            id: m.id,
            category: 'memory',
            title: m.title,
            subtitle: m.content.substring(0, 100),
            badge: m.category || 'memory',
            icon: <BookOpen className="w-4 h-4 text-amber-400" />,
          });
        }
      }
    }

    return items;
  }, [query, activeCategory, assets, entities, collections, memories]);

  // Keep selected index within bounds
  useEffect(() => {
    setSelectedIndex(0);
  }, [results]);

  const handleSelectItem = (item: SearchResultItem) => {
    onClose();
    switch (item.category) {
      case 'asset':
        onNavigate('assets', item.id);
        break;
      case 'entity':
        onNavigate('entities', item.id);
        break;
      case 'collection':
        onNavigate('collections', item.id);
        break;
      case 'memory':
        onNavigate('memories', item.id);
        break;
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev < results.length - 1 ? prev + 1 : prev));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : prev));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (results[selectedIndex]) {
        handleSelectItem(results[selectedIndex]);
      }
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-start justify-center pt-20 px-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-2xl bg-neutral-900 border border-neutral-700/80 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[75vh] animate-in fade-in zoom-in-95 duration-150"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {/* Search Input Bar */}
        <div className="relative flex items-center border-b border-neutral-800 px-4 py-3">
          <Search className="w-4 h-4 text-neutral-400 mr-3 shrink-0" />
          <input
            ref={inputRef}
            type="text"
            placeholder={t.search.inputPlaceholder}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="w-full bg-transparent text-sm text-neutral-100 placeholder-neutral-500 focus:outline-none"
          />
          {query ? (
            <button
              onClick={() => setQuery('')}
              className="p-1 rounded text-neutral-400 hover:text-neutral-200"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          ) : (
            <kbd className="text-[10px] text-neutral-400 bg-neutral-800 px-1.5 py-0.5 rounded font-mono border border-neutral-700">
              ESC
            </kbd>
          )}
        </div>

        {/* Filter Category Tabs */}
        <div className="flex items-center space-x-1 px-4 py-2 bg-neutral-900/90 border-b border-neutral-800 text-xs overflow-x-auto">
          <button
            onClick={() => setActiveCategory('all')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              activeCategory === 'all'
                ? 'bg-neutral-800 text-neutral-100 font-medium'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            {t.search.allTab}
          </button>
          <button
            onClick={() => setActiveCategory('assets')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              activeCategory === 'assets'
                ? 'bg-sky-500/20 text-sky-300 font-medium'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            {t.search.assetsTab} ({assets.length})
          </button>
          <button
            onClick={() => setActiveCategory('entities')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              activeCategory === 'entities'
                ? 'bg-indigo-500/20 text-indigo-300 font-medium'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            {t.search.entitiesTab} ({entities.length})
          </button>
          <button
            onClick={() => setActiveCategory('collections')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              activeCategory === 'collections'
                ? 'bg-purple-500/20 text-purple-300 font-medium'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            {t.search.collectionsTab} ({collections.length})
          </button>
          <button
            onClick={() => setActiveCategory('memories')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              activeCategory === 'memories'
                ? 'bg-amber-500/20 text-amber-300 font-medium'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            {t.search.memoriesTab} ({memories.length})
          </button>
        </div>

        {/* Results List */}
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {results.length === 0 ? (
            <div className="p-10 text-center space-y-2">
              <Sparkles className="w-8 h-8 text-neutral-600 mx-auto" />
              <p className="text-xs font-medium text-neutral-300">{t.search.emptyTitle}</p>
              <p className="text-[11px] text-neutral-500">{t.search.emptyDesc}</p>
            </div>
          ) : (
            results.map((item, index) => {
              const isSelected = index === selectedIndex;
              return (
                <div
                  key={`${item.category}-${item.id}`}
                  onClick={() => handleSelectItem(item)}
                  onMouseEnter={() => setSelectedIndex(index)}
                  className={`flex items-center justify-between px-3 py-2.5 rounded-xl cursor-pointer transition-colors ${
                    isSelected
                      ? 'bg-neutral-800 text-neutral-100 border border-neutral-700/60'
                      : 'hover:bg-neutral-850 text-neutral-300 border border-transparent'
                  }`}
                >
                  <div className="flex items-center space-x-3 overflow-hidden">
                    <div className="p-2 rounded-lg bg-neutral-800/80 border border-neutral-700/50 shrink-0">
                      {item.icon}
                    </div>
                    <div className="overflow-hidden">
                      <div className="flex items-center space-x-2">
                        <span className="text-xs font-medium truncate">{item.title}</span>
                        {item.badge && (
                          <span className="text-[10px] px-1.5 py-0.2 rounded bg-neutral-700/60 text-neutral-400 font-mono">
                            {item.badge}
                          </span>
                        )}
                      </div>
                      <p className="text-[11px] text-neutral-500 truncate mt-0.5">{item.subtitle}</p>
                    </div>
                  </div>

                  <div className="flex items-center space-x-2 shrink-0 ml-3">
                    <span className="text-[10px] text-neutral-500 capitalize">{item.category}</span>
                    <ArrowRight className={`w-3.5 h-3.5 ${isSelected ? 'text-indigo-400' : 'text-neutral-600'}`} />
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer shortcuts hint */}
        <div className="px-4 py-2 border-t border-neutral-800 bg-neutral-900/50 flex items-center justify-between text-[11px] text-neutral-500">
          <span>{t.search.shortcutHint}</span>
          <span className="text-neutral-400 font-mono">{results.length} 结果</span>
        </div>
      </div>
    </div>
  );
};

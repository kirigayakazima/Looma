import React, { useState, useEffect, useMemo } from 'react';
import { Asset, Entity, Memory, TimelineItem } from '../types';
import {
  Calendar,
  Layers,
  FileText,
  BookOpen,
  Search,
  ExternalLink,
  FolderOpen,
  ImageIcon,
  Video,
  Music,
  Code,
  Archive,
  Box,
  FileQuestion,
  Clock,
  X,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { MarkdownRenderer } from './MarkdownRenderer';

interface TimelineViewProps {
  assets: Asset[];
  entities: Entity[];
  memories: Memory[];
  onRefresh: () => Promise<void>;
  onNavigateTab: (tab: 'home' | 'assets' | 'entities' | 'collections' | 'timeline' | 'memories' | 'settings') => void;
}

export const TimelineView: React.FC<TimelineViewProps> = ({
  assets,
  entities,
  memories,
  onNavigateTab,
}) => {
  const [timelineItems, setTimelineItems] = useState<TimelineItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedType, setSelectedType] = useState<'all' | 'asset' | 'entity' | 'memory'>('all');
  const [selectedYear, setSelectedYear] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Selected item modal detail
  const [activeItem, setActiveItem] = useState<TimelineItem | null>(null);
  const [activeAssetPreview, setActiveAssetPreview] = useState<string | null>(null);

  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  const loadTimeline = async () => {
    setLoading(true);
    try {
      const items = await safeInvoke<TimelineItem[]>('query_timeline', {
        filter: {
          item_types: selectedType === 'all' ? undefined : [selectedType],
        },
      });

      if (items) {
        setTimelineItems(items);
      } else {
        // Fallback aggregation client-side if offline / web preview
        const combined: TimelineItem[] = [];

        assets.forEach((a) => {
          const p = a.path || a.id;
          const fname = p.split(/[\\/]/).pop() || a.id;
          combined.push({
            id: a.id,
            item_type: 'asset',
            timestamp: a.modified_at || a.created_at,
            title: fname,
            description: a.path,
            badge: a.kind,
            metadata: (a.metadata as Record<string, unknown>) || {},
          });
        });

        entities.forEach((e) => {
          combined.push({
            id: e.id,
            item_type: 'entity',
            timestamp: e.created_at,
            title: e.title,
            description: e.description,
            badge: e.entity_type,
            metadata: e.properties,
          });
        });

        memories.forEach((m) => {
          combined.push({
            id: m.id,
            item_type: 'memory',
            timestamp: m.recorded_at,
            title: m.title,
            description: m.content,
            badge: m.category,
            metadata: m.metadata,
          });
        });

        combined.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
        setTimelineItems(combined);
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTimeline();
  }, [selectedType, assets.length, entities.length, memories.length]);

  // Load preview when active asset item selected
  useEffect(() => {
    if (activeItem && activeItem.item_type === 'asset') {
      const ast = assets.find((a) => a.id === activeItem.id);
      if (ast && ast.kind === 'image' && ast.path) {
        safeInvoke<string>('read_asset_preview', { path: ast.path }).then((p) => {
          setActiveAssetPreview(p);
        });
      } else {
        setActiveAssetPreview(null);
      }
    } else {
      setActiveAssetPreview(null);
    }
  }, [activeItem, assets]);

  // Available years from timeline items
  const availableYears = useMemo(() => {
    const years = new Set<string>();
    timelineItems.forEach((item) => {
      const y = new Date(item.timestamp).getFullYear().toString();
      if (!isNaN(parseInt(y))) {
        years.add(y);
      }
    });
    return Array.from(years).sort((a, b) => b.localeCompare(a));
  }, [timelineItems]);

  // Filter items by search query and year
  const filteredItems = useMemo(() => {
    return timelineItems.filter((item) => {
      if (selectedYear !== 'all') {
        const y = new Date(item.timestamp).getFullYear().toString();
        if (y !== selectedYear) return false;
      }

      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const tMatch = item.title.toLowerCase().includes(q);
        const dMatch = (item.description || '').toLowerCase().includes(q);
        const bMatch = (item.badge || '').toLowerCase().includes(q);
        if (!tMatch && !dMatch && !bMatch) return false;
      }

      return true;
    });
  }, [timelineItems, selectedYear, searchQuery]);

  // Group items by Year and Month: { "2026": { "10月": [...] } }
  const groupedTimeline = useMemo(() => {
    const groups: Record<string, Record<string, TimelineItem[]>> = {};

    filteredItems.forEach((item) => {
      const d = new Date(item.timestamp);
      const year = isNaN(d.getFullYear()) ? 'Unknown' : d.getFullYear().toString();
      const month = isNaN(d.getMonth())
        ? 'Unknown'
        : `${(d.getMonth() + 1).toString().padStart(2, '0')}月`;

      if (!groups[year]) groups[year] = {};
      if (!groups[year][month]) groups[year][month] = [];
      groups[year][month].push(item);
    });

    return groups;
  }, [filteredItems]);

  const getItemIcon = (itemType: string, badge?: string | null) => {
    const cls = 'w-3.5 h-3.5';
    switch (itemType) {
      case 'entity':
        return <Layers className={`${cls} text-indigo-400`} />;
      case 'memory':
        return <BookOpen className={`${cls} text-amber-400`} />;
      case 'asset':
        switch (badge) {
          case 'image':
            return <ImageIcon className={`${cls} text-sky-400`} />;
          case 'video':
            return <Video className={`${cls} text-purple-400`} />;
          case 'audio':
            return <Music className={`${cls} text-emerald-400`} />;
          case 'code':
            return <Code className={`${cls} text-amber-400`} />;
          case 'document':
            return <FileText className={`${cls} text-blue-400`} />;
          case 'archive':
            return <Archive className={`${cls} text-orange-400`} />;
          case 'model3d':
            return <Box className={`${cls} text-rose-400`} />;
          default:
            return <FileQuestion className={`${cls} text-neutral-400`} />;
        }
      default:
        return <Clock className={`${cls} text-neutral-400`} />;
    }
  };

  const getItemTypeBadge = (item: TimelineItem) => {
    switch (item.item_type) {
      case 'asset':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] bg-sky-500/10 text-sky-400 font-mono border border-sky-500/20">
            {item.badge || 'Asset'}
          </span>
        );
      case 'entity':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] bg-indigo-500/10 text-indigo-400 font-mono border border-indigo-500/20">
            {item.badge || 'Entity'}
          </span>
        );
      case 'memory':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/10 text-amber-400 font-mono border border-amber-500/20">
            {item.badge || 'Memory'}
          </span>
        );
    }
  };

  const handleRevealExplorer = async (path: string | null) => {
    if (!path) return;
    await safeInvoke('open_in_file_manager', { path });
  };

  return (
    <div className="space-y-5 max-w-5xl">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <h3 className="text-base font-semibold text-neutral-100 flex items-center space-x-2">
            <span>时光轴 (Timeline)</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-neutral-800 text-neutral-400 font-normal">
              {filteredItems.length} 事件
            </span>
          </h3>
          <p className="text-xs text-neutral-400">
            按时间顺序聚合您的数字资产、实体概念变迁与记忆手记
          </p>
        </div>

        {/* View switcher */}
        <div className="flex items-center space-x-2">
          <button
            onClick={() => onNavigateTab('memories')}
            className="text-xs px-2.5 py-1.5 rounded-lg bg-neutral-850 hover:bg-neutral-800 text-neutral-300 border border-neutral-800 transition-colors"
          >
            撰写新记录
          </button>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-3 bg-neutral-900/60 border border-neutral-800 p-3 rounded-xl">
        <div className="relative flex-1">
          <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
          <input
            type="text"
            placeholder="在时光轴中搜索事件、标题或标签..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-neutral-800/80 border border-neutral-700/60 rounded-lg pl-8 pr-3 py-1.5 text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-indigo-500"
          />
        </div>

        {/* Type Filter Buttons */}
        <div className="flex items-center space-x-1 text-xs">
          <button
            onClick={() => setSelectedType('all')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              selectedType === 'all'
                ? 'bg-neutral-800 text-neutral-100 font-medium border border-neutral-700'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            全部
          </button>
          <button
            onClick={() => setSelectedType('asset')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              selectedType === 'asset'
                ? 'bg-sky-500/20 text-sky-300 font-medium border border-sky-500/30'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            数字资产
          </button>
          <button
            onClick={() => setSelectedType('entity')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              selectedType === 'entity'
                ? 'bg-indigo-500/20 text-indigo-300 font-medium border border-indigo-500/30'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            实体概念
          </button>
          <button
            onClick={() => setSelectedType('memory')}
            className={`px-2.5 py-1 rounded-md transition-colors ${
              selectedType === 'memory'
                ? 'bg-amber-500/20 text-amber-300 font-medium border border-amber-500/30'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            记忆手记
          </button>

          {/* Year selector */}
          {availableYears.length > 0 && (
            <select
              value={selectedYear}
              onChange={(e) => setSelectedYear(e.target.value)}
              className="bg-neutral-800 border border-neutral-700 rounded-md px-2 py-1 text-xs text-neutral-200 focus:outline-none"
            >
              <option value="all">全部年份</option>
              {availableYears.map((y) => (
                <option key={y} value={y}>
                  {y} 年
                </option>
              ))}
            </select>
          )}
        </div>
      </div>

      {/* Main Timeline Stream */}
      {loading ? (
        <div className="p-12 text-center text-xs text-neutral-500">正在编织时光轴...</div>
      ) : filteredItems.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <Calendar className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">暂无时光轴事件</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            当您扫描索引本地文件、创建实体概念或记录记忆手记时，它们都会自动呈现在此时光轴上。
          </p>
        </div>
      ) : (
        <div className="space-y-8 pl-2">
          {Object.keys(groupedTimeline)
            .sort((a, b) => b.localeCompare(a))
            .map((year) => (
              <div key={year} className="space-y-4">
                {/* Year Header */}
                <div className="flex items-center space-x-3">
                  <div className="px-3 py-1 rounded-full bg-neutral-800 border border-neutral-700 font-mono text-xs font-bold text-neutral-200 tracking-wider">
                    {year}
                  </div>
                  <div className="flex-1 h-px bg-neutral-800" />
                </div>

                {/* Months */}
                {Object.keys(groupedTimeline[year])
                  .sort((a, b) => b.localeCompare(a))
                  .map((month) => (
                    <div key={`${year}-${month}`} className="ml-4 space-y-3">
                      <div className="flex items-center space-x-2 text-xs font-medium text-neutral-400">
                        <span className="w-2 h-2 rounded-full bg-neutral-600" />
                        <span>{month}</span>
                      </div>

                      {/* Items Stream with Vertical Connector */}
                      <div className="relative border-l border-neutral-800/80 ml-1 pl-6 space-y-3">
                        {groupedTimeline[year][month].map((item) => {
                          const dateObj = new Date(item.timestamp);
                          const dateDisplay = isNaN(dateObj.getTime())
                            ? ''
                            : `${dateObj.getDate()}日 ${dateObj
                                .getHours()
                                .toString()
                                .padStart(2, '0')}:${dateObj
                                .getMinutes()
                                .toString()
                                .padStart(2, '0')}`;

                          return (
                            <div
                              key={`${item.item_type}-${item.id}`}
                              onClick={() => setActiveItem(item)}
                              className="relative group p-3.5 rounded-xl bg-neutral-900 border border-neutral-800/90 hover:border-neutral-700 hover:bg-neutral-850/80 transition-all cursor-pointer shadow-sm"
                            >
                              {/* Connector dot on the timeline line */}
                              <div className="absolute -left-[31px] top-4.5 w-2.5 h-2.5 rounded-full bg-neutral-700 border-2 border-neutral-950 group-hover:bg-indigo-400 transition-colors" />

                              <div className="flex items-start justify-between gap-3">
                                <div className="flex items-start space-x-3 overflow-hidden">
                                  <div className="p-2 rounded-lg bg-neutral-800/80 border border-neutral-700/50 shrink-0 mt-0.5">
                                    {getItemIcon(item.item_type, item.badge)}
                                  </div>
                                  <div className="overflow-hidden">
                                    <div className="flex items-center space-x-2">
                                      <h4 className="text-xs font-semibold text-neutral-200 truncate group-hover:text-indigo-300 transition-colors">
                                        {item.title}
                                      </h4>
                                      {getItemTypeBadge(item)}
                                    </div>

                                    {item.description && (
                                      <p className="text-[11px] text-neutral-400 mt-1 line-clamp-2 leading-relaxed">
                                        {item.description}
                                      </p>
                                    )}
                                  </div>
                                </div>

                                <div className="text-[10px] text-neutral-500 font-mono shrink-0">
                                  {dateDisplay}
                                </div>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  ))}
              </div>
            ))}
        </div>
      )}

      {/* Item Detail Modal */}
      {activeItem && (
        <div
          className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4"
          onClick={() => setActiveItem(null)}
        >
          <div
            className="w-full max-w-xl bg-neutral-900 border border-neutral-700 rounded-2xl shadow-2xl overflow-hidden p-6 space-y-4 animate-in fade-in zoom-in-95 duration-150"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between border-b border-neutral-800 pb-3">
              <div className="flex items-center space-x-2">
                <div className="p-2 rounded-lg bg-neutral-800 border border-neutral-700">
                  {getItemIcon(activeItem.item_type, activeItem.badge)}
                </div>
                <div>
                  <h3 className="text-sm font-bold text-neutral-100">{activeItem.title}</h3>
                  <div className="flex items-center space-x-2 mt-0.5">
                    {getItemTypeBadge(activeItem)}
                    <span className="text-[10px] text-neutral-500 font-mono">
                      {new Date(activeItem.timestamp).toLocaleString()}
                    </span>
                  </div>
                </div>
              </div>
              <button
                onClick={() => setActiveItem(null)}
                className="text-neutral-400 hover:text-neutral-200"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Asset preview if image */}
            {activeAssetPreview && (
              <div className="rounded-xl overflow-hidden bg-neutral-950 border border-neutral-800 flex items-center justify-center max-h-64">
                <img
                  src={activeAssetPreview}
                  alt={activeItem.title}
                  className="max-h-64 object-contain"
                />
              </div>
            )}

            {/* Content / Markdown / Properties */}
            <div className="max-h-80 overflow-y-auto space-y-3">
              {activeItem.item_type === 'memory' && activeItem.description ? (
                <div className="p-3 rounded-lg bg-neutral-950/60 border border-neutral-800/80">
                  <MarkdownRenderer content={activeItem.description} />
                </div>
              ) : activeItem.item_type === 'entity' ? (
                <div className="space-y-2">
                  {activeItem.description && (
                    <p className="text-xs text-neutral-300">{activeItem.description}</p>
                  )}
                  {activeItem.metadata && Object.keys(activeItem.metadata).length > 0 && (
                    <div className="p-3 rounded-lg bg-neutral-950 border border-neutral-800 space-y-1.5">
                      <p className="text-[10px] font-semibold text-neutral-400 uppercase tracking-wider">
                        自定义扩展属性
                      </p>
                      <div className="grid grid-cols-2 gap-2 text-xs">
                        {Object.entries(activeItem.metadata).map(([k, v]) => (
                          <div key={k} className="flex space-x-1">
                            <span className="text-neutral-500">{k}:</span>
                            <span className="text-neutral-200 font-mono truncate">{String(v)}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              ) : (
                <div className="space-y-1 text-xs">
                  <p className="text-neutral-400 break-all font-mono text-[11px]">
                    {activeItem.description}
                  </p>
                </div>
              )}
            </div>

            {/* Modal actions */}
            <div className="flex items-center justify-between border-t border-neutral-800 pt-3 text-xs">
              {activeItem.item_type === 'asset' && activeItem.description ? (
                <button
                  onClick={() => handleRevealExplorer(activeItem.description)}
                  className="flex items-center space-x-1 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors"
                >
                  <FolderOpen className="w-3.5 h-3.5" />
                  <span>在系统资源管理器中定位</span>
                </button>
              ) : (
                <div />
              )}

              <button
                onClick={() => {
                  const targetTab =
                    activeItem.item_type === 'asset'
                      ? 'assets'
                      : activeItem.item_type === 'entity'
                      ? 'entities'
                      : 'memories';
                  setActiveItem(null);
                  onNavigateTab(targetTab);
                }}
                className="flex items-center space-x-1 px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white font-medium transition-colors"
              >
                <span>前往该资源模块</span>
                <ExternalLink className="w-3 h-3" />
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

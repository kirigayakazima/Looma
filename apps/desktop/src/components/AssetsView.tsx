import React, { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  FolderArchive,
  FileText,
  Image as ImageIcon,
  Video,
  Music,
  Archive,
  Code,
  Box,
  FileQuestion,
  FolderOpen,
  RefreshCw,
  Search,
  LayoutGrid,
  List,
  ExternalLink,
  Trash2,
  Copy,
  Check,
  X,
  Play,
  CheckCircle2,
  Loader2,
} from 'lucide-react';
import { Asset, ScanProgress, ScanSummary } from '../types';
import { useI18n } from '../i18n';

interface AssetsViewProps {
  assets: Asset[];
  onRefresh: () => Promise<void> | void;
}

export const AssetsView: React.FC<AssetsViewProps> = ({ assets, onRefresh }) => {
  const { t } = useI18n();

  // View & Filter states
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [selectedKind, setSelectedKind] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Scanner modal & progress states
  const [isScanModalOpen, setIsScanModalOpen] = useState(false);
  const [scanPath, setScanPath] = useState('');
  const [computeHash, setComputeHash] = useState(true);
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [scanSummary, setScanSummary] = useState<ScanSummary | null>(null);

  // Asset detail modal
  const [activeAsset, setActiveAsset] = useState<Asset | null>(null);
  const [assetPreview, setAssetPreview] = useState<string | null>(null);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  // Helper for safe IPC invocation
  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  // Listen to scan-progress events from Rust
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    listen<ScanProgress>('scan-progress', (event) => {
      setScanProgress(event.payload);
    }).then((fn) => {
      unlistenFn = fn;
    }).catch((err) => {
      console.warn('Could not register scan-progress listener (probably in browser mode):', err);
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, []);

  // Fetch image preview when active asset changes
  useEffect(() => {
    if (activeAsset && activeAsset.kind === 'image' && activeAsset.path) {
      setLoadingPreview(true);
      setAssetPreview(null);
      safeInvoke<string>('read_asset_preview', { path: activeAsset.path }).then((preview) => {
        setAssetPreview(preview);
        setLoadingPreview(false);
      });
    } else {
      setAssetPreview(null);
      setLoadingPreview(false);
    }
  }, [activeAsset]);

  // Pick folder using native dialog
  const handlePickFolder = async () => {
    const selected = await safeInvoke<string | null>('pick_folder');
    if (selected) {
      setScanPath(selected);
    }
  };

  // Run directory scan
  const handleStartScan = async () => {
    if (!scanPath.trim() || isScanning) return;
    setIsScanning(true);
    setScanProgress(null);
    setScanSummary(null);

    try {
      const summary = await safeInvoke<ScanSummary>('scan_directory', {
        path: scanPath.trim(),
        computeHash,
      });

      if (summary) {
        setScanSummary(summary);
      }
      await onRefresh();
    } finally {
      setIsScanning(false);
    }
  };

  // Reveal in explorer
  const handleReveal = async (path: string | null, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    if (!path) return;
    await safeInvoke('open_in_file_manager', { path });
  };

  // Delete asset record
  const handleDelete = async (id: string, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    if (!window.confirm(t.assets.deleteConfirm)) return;
    await safeInvoke('delete_asset', { id });
    if (activeAsset?.id === id) {
      setActiveAsset(null);
    }
    await onRefresh();
  };

  // Copy to clipboard
  const handleCopy = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  // Format bytes
  const formatSize = (bytes: number | null) => {
    if (bytes === null || bytes === undefined) return '—';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  // Kind icon helper
  const getKindIcon = (kind: string, className = 'w-4 h-4') => {
    switch (kind) {
      case 'image':
        return <ImageIcon className={`${className} text-sky-400`} />;
      case 'video':
        return <Video className={`${className} text-purple-400`} />;
      case 'audio':
        return <Music className={`${className} text-emerald-400`} />;
      case 'document':
        return <FileText className={`${className} text-blue-400`} />;
      case 'code':
        return <Code className={`${className} text-amber-400`} />;
      case 'archive':
        return <Archive className={`${className} text-orange-400`} />;
      case 'model3d':
        return <Box className={`${className} text-rose-400`} />;
      default:
        return <FileQuestion className={`${className} text-neutral-400`} />;
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
      case 'document':
        return t.assets.kindDocument;
      case 'code':
        return t.assets.kindCode;
      case 'archive':
        return t.assets.kindArchive;
      case 'model3d':
        return t.assets.kindModel3d;
      default:
        return t.assets.kindOther;
    }
  };

  // Filter & Search assets
  const filteredAssets = useMemo(() => {
    return assets.filter((asset) => {
      // Kind filter
      if (selectedKind !== 'all' && asset.kind !== selectedKind) {
        return false;
      }
      // Search query filter
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const p = (asset.path || '').toLowerCase();
        const id = asset.id.toLowerCase();
        if (!p.includes(q) && !id.includes(q)) {
          return false;
        }
      }
      return true;
    });
  }, [assets, selectedKind, searchQuery]);

  const kindOptions: { key: string; label: string }[] = [
    { key: 'all', label: t.assets.filterAll },
    { key: 'image', label: t.assets.kindImage },
    { key: 'video', label: t.assets.kindVideo },
    { key: 'audio', label: t.assets.kindAudio },
    { key: 'document', label: t.assets.kindDocument },
    { key: 'code', label: t.assets.kindCode },
    { key: 'archive', label: t.assets.kindArchive },
    { key: 'model3d', label: t.assets.kindModel3d },
  ];

  return (
    <div className="space-y-5 max-w-6xl">
      {/* Header Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold text-neutral-100 flex items-center space-x-2">
            <span>{t.assets.title}</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-neutral-800 text-neutral-400 font-normal">
              {assets.length}
            </span>
          </h3>
          <p className="text-xs text-neutral-400 mt-0.5">{t.assets.subtitle}</p>
        </div>

        <div className="flex items-center space-x-2">
          {/* Scan directory button */}
          <button
            onClick={() => setIsScanModalOpen(true)}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium shadow-sm transition-colors"
          >
            <FolderOpen className="w-3.5 h-3.5" />
            <span>{t.assets.scanFolderBtn}</span>
          </button>

          {/* Refresh button */}
          <button
            onClick={onRefresh}
            className="p-1.5 rounded-lg bg-neutral-850 hover:bg-neutral-800 text-neutral-300 border border-neutral-800 transition-colors"
            title={t.common.refresh}
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>

          {/* View Mode Toggle */}
          <div className="flex items-center bg-neutral-900 border border-neutral-800 rounded-lg p-0.5">
            <button
              onClick={() => setViewMode('grid')}
              className={`p-1 rounded ${
                viewMode === 'grid'
                  ? 'bg-neutral-800 text-neutral-200'
                  : 'text-neutral-500 hover:text-neutral-300'
              }`}
              title={t.assets.viewGrid}
            >
              <LayoutGrid className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setViewMode('list')}
              className={`p-1 rounded ${
                viewMode === 'list'
                  ? 'bg-neutral-800 text-neutral-200'
                  : 'text-neutral-500 hover:text-neutral-300'
              }`}
              title={t.assets.viewList}
            >
              <List className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-3 bg-neutral-900/60 border border-neutral-800/80 p-3 rounded-xl">
        {/* Kind Filters */}
        <div className="flex items-center space-x-1 overflow-x-auto pb-1 md:pb-0 scrollbar-none">
          {kindOptions.map((opt) => (
            <button
              key={opt.key}
              onClick={() => setSelectedKind(opt.key)}
              className={`text-xs px-2.5 py-1 rounded-md transition-colors whitespace-nowrap ${
                selectedKind === opt.key
                  ? 'bg-neutral-800 text-neutral-100 font-medium'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {/* Search Input */}
        <div className="relative min-w-[240px]">
          <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.assets.searchPlaceholder}
            className="w-full pl-8 pr-3 py-1 text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-neutral-700"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery('')}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-500 hover:text-neutral-300"
            >
              <X className="w-3 h-3" />
            </button>
          )}
        </div>
      </div>

      {/* Asset Grid / List Display */}
      {filteredAssets.length === 0 ? (
        <div className="p-12 rounded-xl bg-neutral-900 border border-neutral-800 text-center space-y-3">
          <FolderArchive className="w-10 h-10 text-neutral-600 mx-auto" />
          <h4 className="text-sm font-medium text-neutral-300">{t.assets.emptyTitle}</h4>
          <p className="text-xs text-neutral-500 max-w-sm mx-auto">
            {t.assets.emptyDesc}
          </p>
          <button
            onClick={() => setIsScanModalOpen(true)}
            className="inline-flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-emerald-600/90 hover:bg-emerald-600 text-white text-xs font-medium transition-colors"
          >
            <FolderOpen className="w-3.5 h-3.5" />
            <span>{t.assets.scanFolderBtn}</span>
          </button>
        </div>
      ) : viewMode === 'grid' ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
          {filteredAssets.map((asset) => {
            const fileName = asset.path?.split(/[\\/]/).pop() || asset.id;
            const parentDir = asset.path?.substring(0, asset.path.lastIndexOf(fileName)) || '';
            const isMissing = asset.status === 'missing';

            return (
              <div
                key={asset.id}
                onClick={() => setActiveAsset(asset)}
                className={`group cursor-pointer p-3.5 rounded-xl bg-neutral-900 border transition-all duration-150 flex flex-col justify-between hover:shadow-lg ${
                  isMissing
                    ? 'border-red-900/50 bg-red-950/10'
                    : 'border-neutral-800 hover:border-neutral-700 hover:bg-neutral-850/80'
                }`}
              >
                <div>
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <div className="p-2 rounded-lg bg-neutral-950 border border-neutral-800/80 group-hover:border-neutral-700 transition-colors">
                      {getKindIcon(asset.kind, 'w-5 h-5')}
                    </div>
                    <div className="flex items-center space-x-1">
                      {isMissing && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-red-950/80 text-red-400 border border-red-800/50">
                          {t.assets.statusMissing}
                        </span>
                      )}
                      <button
                        onClick={(e) => handleReveal(asset.path, e)}
                        className="opacity-0 group-hover:opacity-100 p-1 text-neutral-400 hover:text-neutral-100 transition-opacity"
                        title={t.assets.revealInExplorer}
                      >
                        <ExternalLink className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>

                  <h5
                    className="text-xs font-medium text-neutral-200 truncate group-hover:text-white transition-colors"
                    title={fileName}
                  >
                    {fileName}
                  </h5>
                  <p
                    className="text-[11px] text-neutral-500 truncate mt-0.5 font-mono"
                    title={parentDir}
                  >
                    {parentDir || '—'}
                  </p>
                </div>

                <div className="mt-3 pt-2 border-t border-neutral-800/60 flex items-center justify-between text-[10px] text-neutral-400">
                  <span className="uppercase tracking-wider font-semibold text-[9px] text-neutral-400">
                    {getKindLabel(asset.kind)}
                  </span>
                  <span>{formatSize(asset.size)}</span>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        /* List View */
        <div className="rounded-xl border border-neutral-800 bg-neutral-900 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs text-neutral-300">
              <thead className="bg-neutral-950 text-neutral-400 border-b border-neutral-800 text-[11px]">
                <tr>
                  <th className="py-2.5 px-3 font-medium">{t.common.type}</th>
                  <th className="py-2.5 px-3 font-medium">{t.common.title}</th>
                  <th className="py-2.5 px-3 font-medium">{t.assets.pathLabel}</th>
                  <th className="py-2.5 px-3 font-medium">{t.assets.sizeLabel}</th>
                  <th className="py-2.5 px-3 font-medium">{t.assets.statusLabel}</th>
                  <th className="py-2.5 px-3 font-medium text-right">操作</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-neutral-800/60">
                {filteredAssets.map((asset) => {
                  const fileName = asset.path?.split(/[\\/]/).pop() || asset.id;
                  const isMissing = asset.status === 'missing';

                  return (
                    <tr
                      key={asset.id}
                      onClick={() => setActiveAsset(asset)}
                      className="hover:bg-neutral-850/60 cursor-pointer transition-colors"
                    >
                      <td className="py-2.5 px-3 whitespace-nowrap">
                        <div className="flex items-center space-x-1.5">
                          {getKindIcon(asset.kind, 'w-3.5 h-3.5')}
                          <span className="text-[10px] uppercase text-neutral-400">
                            {getKindLabel(asset.kind)}
                          </span>
                        </div>
                      </td>
                      <td className="py-2.5 px-3 font-medium text-neutral-200 max-w-[200px] truncate">
                        {fileName}
                      </td>
                      <td className="py-2.5 px-3 text-neutral-500 font-mono text-[11px] max-w-[320px] truncate">
                        {asset.path || '—'}
                      </td>
                      <td className="py-2.5 px-3 text-neutral-400 whitespace-nowrap">
                        {formatSize(asset.size)}
                      </td>
                      <td className="py-2.5 px-3 whitespace-nowrap">
                        {isMissing ? (
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-red-950/80 text-red-400 border border-red-800/50">
                            {t.assets.statusMissing}
                          </span>
                        ) : (
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-950/80 text-emerald-400 border border-emerald-800/50">
                            {t.assets.statusActive}
                          </span>
                        )}
                      </td>
                      <td className="py-2.5 px-3 text-right whitespace-nowrap">
                        <div className="flex items-center justify-end space-x-1">
                          <button
                            onClick={(e) => handleReveal(asset.path, e)}
                            className="p-1 hover:text-white text-neutral-400 rounded hover:bg-neutral-800 transition-colors"
                            title={t.assets.revealInExplorer}
                          >
                            <ExternalLink className="w-3.5 h-3.5" />
                          </button>
                          <button
                            onClick={(e) => handleDelete(asset.id, e)}
                            className="p-1 hover:text-red-400 text-neutral-500 rounded hover:bg-neutral-800 transition-colors"
                            title={t.assets.deleteAsset}
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Directory Scanner Modal */}
      {isScanModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-lg w-full p-6 shadow-2xl space-y-5">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <FolderOpen className="w-5 h-5 text-emerald-400" />
                <h4 className="text-base font-semibold text-neutral-100">
                  {t.assets.scanFolderBtn}
                </h4>
              </div>
              <button
                onClick={() => {
                  if (!isScanning) setIsScanModalOpen(false);
                }}
                disabled={isScanning}
                className="text-neutral-500 hover:text-neutral-300 disabled:opacity-50"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Folder selection */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-neutral-300">
                {t.assets.selectedFolder}
              </label>
              <div className="flex items-center space-x-2">
                <input
                  type="text"
                  value={scanPath}
                  onChange={(e) => setScanPath(e.target.value)}
                  placeholder={t.assets.noFolderSelected}
                  disabled={isScanning}
                  className="flex-1 px-3 py-2 text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-neutral-700 font-mono"
                />
                <button
                  onClick={handlePickFolder}
                  disabled={isScanning}
                  className="px-3 py-2 text-xs font-medium rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 border border-neutral-700 transition-colors whitespace-nowrap disabled:opacity-50"
                >
                  {t.assets.chooseFolder}
                </button>
              </div>
            </div>

            {/* Hash option checkbox */}
            <label className="flex items-start space-x-2.5 cursor-pointer text-xs text-neutral-300">
              <input
                type="checkbox"
                checked={computeHash}
                onChange={(e) => setComputeHash(e.target.checked)}
                disabled={isScanning}
                className="mt-0.5 rounded border-neutral-700 bg-neutral-950 text-emerald-600 focus:ring-0"
              />
              <span className="leading-relaxed text-neutral-400">
                {t.assets.computeHashOption}
              </span>
            </label>

            {/* Live Progress Display */}
            {isScanning && (
              <div className="p-4 rounded-xl bg-neutral-950 border border-neutral-800 space-y-2.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-neutral-300 flex items-center space-x-2">
                    <Loader2 className="w-3.5 h-3.5 animate-spin text-emerald-400" />
                    <span>{t.assets.scanProgressTitle}</span>
                  </span>
                  <span className="font-mono text-emerald-400">
                    {scanProgress?.total_discovered || 0}
                  </span>
                </div>
                {scanProgress?.current_file && (
                  <p className="text-[11px] font-mono text-neutral-500 truncate">
                    {scanProgress.current_file}
                  </p>
                )}
                <div className="grid grid-cols-3 gap-2 text-[10px] text-neutral-400 pt-1 border-t border-neutral-800/80">
                  <div>新增: <span className="text-emerald-400 font-mono">{scanProgress?.new_assets || 0}</span></div>
                  <div>更新: <span className="text-amber-400 font-mono">{scanProgress?.modified_assets || 0}</span></div>
                  <div>未变: <span className="text-neutral-300 font-mono">{scanProgress?.unchanged_assets || 0}</span></div>
                </div>
              </div>
            )}

            {/* Summary Display */}
            {scanSummary && !isScanning && (
              <div className="p-4 rounded-xl bg-emerald-950/20 border border-emerald-800/40 space-y-2 text-xs">
                <div className="flex items-center space-x-1.5 text-emerald-400 font-medium">
                  <CheckCircle2 className="w-4 h-4" />
                  <span>{t.assets.scanSummaryTitle} ({scanSummary.duration_ms} ms)</span>
                </div>
                <div className="text-neutral-300 text-[11px] space-y-1">
                  <p>
                    {t.assets.scanSummaryStats
                      .replace('{total}', String(scanSummary.total_scanned))
                      .replace('{new}', String(scanSummary.new_assets))
                      .replace('{modified}', String(scanSummary.modified_assets))
                      .replace('{unchanged}', String(scanSummary.unchanged_assets))}
                  </p>
                  {scanSummary.missing_assets > 0 && (
                    <p className="text-amber-400">
                      {t.assets.scanMissingNote.replace('{missing}', String(scanSummary.missing_assets))}
                    </p>
                  )}
                </div>
              </div>
            )}

            {/* Modal Actions */}
            <div className="flex items-center justify-end space-x-2 pt-2">
              <button
                onClick={() => setIsScanModalOpen(false)}
                disabled={isScanning}
                className="px-4 py-2 text-xs rounded-lg text-neutral-400 hover:text-neutral-200 transition-colors disabled:opacity-50"
              >
                {scanSummary ? t.assets.closeBtn : t.common.cancel}
              </button>
              <button
                onClick={handleStartScan}
                disabled={!scanPath.trim() || isScanning}
                className="flex items-center space-x-1.5 px-4 py-2 text-xs font-medium rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white transition-colors disabled:opacity-50 shadow"
              >
                {isScanning ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    <span>{t.assets.scanning}</span>
                  </>
                ) : (
                  <>
                    <Play className="w-3.5 h-3.5" />
                    <span>{t.assets.startScanBtn}</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Asset Detail Drawer / Modal */}
      {activeAsset && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-in fade-in">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-xl w-full p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto">
            <div className="flex items-start justify-between">
              <div className="flex items-center space-x-3">
                <div className="p-2.5 rounded-xl bg-neutral-950 border border-neutral-800">
                  {getKindIcon(activeAsset.kind, 'w-6 h-6')}
                </div>
                <div>
                  <h4 className="text-sm font-semibold text-neutral-100 max-w-md truncate">
                    {activeAsset.path?.split(/[\\/]/).pop() || activeAsset.id}
                  </h4>
                  <div className="flex items-center space-x-2 mt-1">
                    <span className="text-[10px] uppercase tracking-wider font-semibold text-neutral-400">
                      {getKindLabel(activeAsset.kind)}
                    </span>
                    <span className="text-neutral-600">•</span>
                    <span className="text-[11px] text-neutral-400">
                      {formatSize(activeAsset.size)}
                    </span>
                  </div>
                </div>
              </div>
              <button
                onClick={() => setActiveAsset(null)}
                className="text-neutral-500 hover:text-neutral-300"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Thumbnail preview if Image */}
            {activeAsset.kind === 'image' && (
              <div className="rounded-xl border border-neutral-800 bg-neutral-950 overflow-hidden flex items-center justify-center min-h-[160px] max-h-[260px]">
                {loadingPreview ? (
                  <div className="flex items-center space-x-2 text-xs text-neutral-500">
                    <Loader2 className="w-4 h-4 animate-spin text-sky-400" />
                    <span>加载预览...</span>
                  </div>
                ) : assetPreview ? (
                  <img
                    src={assetPreview}
                    alt={activeAsset.id}
                    className="max-h-[260px] w-auto object-contain mx-auto"
                  />
                ) : (
                  <div className="text-xs text-neutral-600">无法直接显示图片缩略图</div>
                )}
              </div>
            )}

            {/* Properties List */}
            <div className="space-y-3 bg-neutral-950 border border-neutral-800/80 rounded-xl p-4 text-xs">
              {/* Path */}
              <div className="space-y-1">
                <div className="text-[11px] text-neutral-500 flex items-center justify-between">
                  <span>{t.assets.pathLabel}</span>
                  <div className="flex items-center space-x-1.5">
                    <button
                      onClick={() => handleCopy(activeAsset.path || '', 'path')}
                      className="text-neutral-400 hover:text-neutral-200 flex items-center space-x-1 text-[10px]"
                    >
                      {copiedField === 'path' ? (
                        <Check className="w-3 h-3 text-emerald-400" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                      <span>{copiedField === 'path' ? t.assets.copySuccess : '复制'}</span>
                    </button>
                    <button
                      onClick={() => handleReveal(activeAsset.path)}
                      className="text-emerald-400 hover:text-emerald-300 flex items-center space-x-1 text-[10px]"
                    >
                      <ExternalLink className="w-3 h-3" />
                      <span>{t.assets.revealInExplorer}</span>
                    </button>
                  </div>
                </div>
                <div className="font-mono text-neutral-200 break-all bg-neutral-900/60 p-2 rounded border border-neutral-800/60">
                  {activeAsset.path || '—'}
                </div>
              </div>

              {/* SHA-256 */}
              {activeAsset.hash && (
                <div className="space-y-1">
                  <div className="text-[11px] text-neutral-500 flex items-center justify-between">
                    <span>{t.assets.hashLabel}</span>
                    <button
                      onClick={() => handleCopy(activeAsset.hash || '', 'hash')}
                      className="text-neutral-400 hover:text-neutral-200 flex items-center space-x-1 text-[10px]"
                    >
                      {copiedField === 'hash' ? (
                        <Check className="w-3 h-3 text-emerald-400" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                      <span>{copiedField === 'hash' ? t.assets.copySuccess : '复制'}</span>
                    </button>
                  </div>
                  <div className="font-mono text-[11px] text-neutral-300 break-all bg-neutral-900/60 p-2 rounded border border-neutral-800/60">
                    {activeAsset.hash}
                  </div>
                </div>
              )}

              {/* Grid attributes */}
              <div className="grid grid-cols-2 gap-3 pt-2 border-t border-neutral-800/60 text-[11px]">
                <div>
                  <span className="text-neutral-500">{t.assets.mimeLabel}:</span>{' '}
                  <span className="text-neutral-300 font-mono">{activeAsset.mime_type || '—'}</span>
                </div>
                <div>
                  <span className="text-neutral-500">{t.assets.sourceLabel}:</span>{' '}
                  <span className="text-emerald-400 font-medium">{t.assets.sourceLocal}</span>
                </div>
                <div>
                  <span className="text-neutral-500">{t.assets.statusLabel}:</span>{' '}
                  <span
                    className={`font-medium ${
                      activeAsset.status === 'active' ? 'text-emerald-400' : 'text-red-400'
                    }`}
                  >
                    {activeAsset.status === 'active' ? t.assets.statusActive : t.assets.statusMissing}
                  </span>
                </div>
                <div>
                  <span className="text-neutral-500">{t.assets.modifiedLabel}:</span>{' '}
                  <span className="text-neutral-300">
                    {new Date(activeAsset.modified_at).toLocaleString()}
                  </span>
                </div>
              </div>
            </div>

            {/* Actions */}
            <div className="flex items-center justify-between pt-1">
              <button
                onClick={() => handleDelete(activeAsset.id)}
                className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-red-400 hover:bg-red-950/40 text-xs transition-colors border border-red-900/40"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>{t.assets.deleteAsset}</span>
              </button>

              <div className="flex items-center space-x-2">
                <button
                  onClick={() => handleReveal(activeAsset.path)}
                  className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium border border-neutral-700 transition-colors"
                >
                  <FolderOpen className="w-3.5 h-3.5" />
                  <span>{t.assets.revealInExplorer}</span>
                </button>
                <button
                  onClick={() => setActiveAsset(null)}
                  className="px-4 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium transition-colors"
                >
                  {t.assets.closeBtn}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

import React, { useState } from 'react';
import { VaultInfo, BackupResult, RestoreResult, VaultDoctorReport } from '../types';
import {
  ShieldCheck,
  HardDrive,
  Database,
  Globe,
  Check,
  Archive,
  RotateCcw,
  Stethoscope,
  Sparkles,
  AlertTriangle,
  FolderOpen,
  Terminal,
  Cpu,
} from 'lucide-react';
import { useI18n, Locale } from '../i18n';
import { invoke } from '@tauri-apps/api/core';

interface SettingsViewProps {
  vaultInfo: VaultInfo | null;
  onRefresh?: () => Promise<void>;
}

export const SettingsView: React.FC<SettingsViewProps> = ({ vaultInfo, onRefresh }) => {
  const { locale, setLocale, t } = useI18n();

  // Backup & Restore states
  const [isExporting, setIsExporting] = useState(false);
  const [backupResult, setBackupResult] = useState<BackupResult | null>(null);

  const [isRestoring, setIsRestoring] = useState(false);
  const [restoreResult, setRestoreResult] = useState<RestoreResult | null>(null);

  // Doctor states
  const [isCheckingDoctor, setIsCheckingDoctor] = useState(false);
  const [doctorReport, setDoctorReport] = useState<VaultDoctorReport | null>(null);
  const [isCleaning, setIsCleaning] = useState(false);
  const [cleanMessage, setCleanMessage] = useState<string | null>(null);

  const safeInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] '${cmd}' failed:`, err);
      return null;
    }
  };

  const languages: { code: Locale; label: string; desc: string }[] = [
    { code: 'zh-CN', label: t.settings.langZh, desc: '默认语言 (Default)' },
    { code: 'en-US', label: t.settings.langEn, desc: 'English (US)' },
  ];

  // Handle Export Backup
  const handleExportBackup = async () => {
    setIsExporting(true);
    setBackupResult(null);
    try {
      // Allow user to select destination folder, or default
      const chosenDir = await safeInvoke<string | null>('pick_folder');
      const res = await safeInvoke<BackupResult>('export_backup', {
        destinationDir: chosenDir || undefined,
      });
      if (res) {
        setBackupResult(res);
      }
    } finally {
      setIsExporting(false);
    }
  };

  // Handle Restore Backup
  const handleRestoreBackup = async () => {
    const chosenDir = await safeInvoke<string | null>('pick_folder');
    if (!chosenDir) return;

    if (!window.confirm(t.settings.restoreConfirm)) return;

    setIsRestoring(true);
    setRestoreResult(null);
    try {
      const res = await safeInvoke<RestoreResult>('restore_backup', {
        backupPath: chosenDir,
      });
      if (res) {
        setRestoreResult(res);
        if (onRefresh) {
          await onRefresh();
        }
      }
    } finally {
      setIsRestoring(false);
    }
  };

  // Handle Doctor Inspection
  const handleRunDoctor = async () => {
    setIsCheckingDoctor(true);
    setCleanMessage(null);
    try {
      const report = await safeInvoke<VaultDoctorReport>('doctor_inspect');
      if (report) {
        setDoctorReport(report);
      }
    } finally {
      setIsCheckingDoctor(false);
    }
  };

  // Handle Doctor Cleanup & Vacuum
  const handleDoctorCleanup = async () => {
    setIsCleaning(true);
    try {
      const count = await safeInvoke<number>('doctor_cleanup_missing');
      setCleanMessage(
        (t.settings.cleanSuccess || 'Cleanup successful!').replace('{count}', (count ?? 0).toString())
      );
      // Re-inspect
      await handleRunDoctor();
      if (onRefresh) {
        await onRefresh();
      }
    } finally {
      setIsCleaning(false);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  return (
    <div className="space-y-6 max-w-4xl pb-10">
      <div>
        <h3 className="text-base font-semibold text-neutral-100">{t.settings.title}</h3>
        <p className="text-xs text-neutral-400">{t.settings.subtitle}</p>
      </div>

      <div className="space-y-4">
        {/* Vault Backup & Export Section */}
        <div className="p-5 rounded-xl bg-neutral-900 border border-neutral-800 space-y-3">
          <div className="flex items-center space-x-2 text-sm font-medium text-neutral-200">
            <Archive className="w-4 h-4 text-amber-400" />
            <span>{t.settings.backupTitle}</span>
          </div>
          <p className="text-xs text-neutral-400 leading-relaxed">
            {t.settings.backupDesc}
          </p>

          <div className="pt-1">
            <button
              onClick={handleExportBackup}
              disabled={isExporting}
              className="flex items-center space-x-2 px-3.5 py-2 rounded-lg bg-amber-600 hover:bg-amber-500 disabled:opacity-50 text-white text-xs font-medium shadow-sm transition-colors"
            >
              <Archive className="w-3.5 h-3.5" />
              <span>{isExporting ? t.settings.exporting : t.settings.exportBackupBtn}</span>
            </button>
          </div>

          {backupResult && (
            <div className="mt-3 p-3.5 rounded-lg bg-neutral-950 border border-amber-500/30 text-xs space-y-2">
              <div className="flex items-center space-x-1.5 text-amber-400 font-medium">
                <Check className="w-4 h-4 text-emerald-400" />
                <span>{t.settings.exportSuccess}</span>
              </div>
              <p className="font-mono text-[11px] text-neutral-300 bg-neutral-900 p-2 rounded break-all select-text">
                {backupResult.backup_path}
              </p>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] text-neutral-400 pt-1">
                <div>
                  资产清单: <span className="text-neutral-200 font-mono">{backupResult.manifest.assets_count}</span>
                </div>
                <div>
                  实体概念: <span className="text-neutral-200 font-mono">{backupResult.manifest.entities_count}</span>
                </div>
                <div>
                  记忆手记: <span className="text-neutral-200 font-mono">{backupResult.manifest.memories_count}</span>
                </div>
                <div>
                  耗时: <span className="text-neutral-200 font-mono">{backupResult.duration_ms} ms</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Restore Vault Section */}
        <div className="p-5 rounded-xl bg-neutral-900 border border-neutral-800 space-y-3">
          <div className="flex items-center space-x-2 text-sm font-medium text-neutral-200">
            <RotateCcw className="w-4 h-4 text-sky-400" />
            <span>{t.settings.restoreTitle}</span>
          </div>
          <p className="text-xs text-neutral-400 leading-relaxed">
            {t.settings.restoreDesc}
          </p>

          <div className="pt-1">
            <button
              onClick={handleRestoreBackup}
              disabled={isRestoring}
              className="flex items-center space-x-2 px-3.5 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-750 disabled:opacity-50 text-neutral-200 text-xs font-medium border border-neutral-700 transition-colors"
            >
              <FolderOpen className="w-3.5 h-3.5 text-sky-400" />
              <span>{isRestoring ? t.settings.restoring : t.settings.restoreBtn}</span>
            </button>
          </div>

          {restoreResult && (
            <div className="mt-3 p-3.5 rounded-lg bg-neutral-950 border border-sky-500/30 text-xs space-y-1.5">
              <div className="flex items-center space-x-1.5 text-sky-400 font-medium">
                <Check className="w-4 h-4 text-emerald-400" />
                <span>{t.settings.restoreSuccess}</span>
              </div>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] text-neutral-400">
                <div>
                  恢复资产: <span className="text-neutral-200 font-mono">{restoreResult.restored_assets}</span>
                </div>
                <div>
                  恢复实体: <span className="text-neutral-200 font-mono">{restoreResult.restored_entities}</span>
                </div>
                <div>
                  恢复手记: <span className="text-neutral-200 font-mono">{restoreResult.restored_memories}</span>
                </div>
                <div>
                  耗时: <span className="text-neutral-200 font-mono">{restoreResult.duration_ms} ms</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Vault Doctor Section */}
        <div className="p-5 rounded-xl bg-neutral-900 border border-neutral-800 space-y-3">
          <div className="flex items-center space-x-2 text-sm font-medium text-neutral-200">
            <Stethoscope className="w-4 h-4 text-emerald-400" />
            <span>{t.settings.doctorTitle}</span>
          </div>
          <p className="text-xs text-neutral-400 leading-relaxed">
            {t.settings.doctorDesc}
          </p>

          <div className="flex flex-wrap items-center gap-2 pt-1">
            <button
              onClick={handleRunDoctor}
              disabled={isCheckingDoctor}
              className="flex items-center space-x-2 px-3.5 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white text-xs font-medium shadow-sm transition-colors"
            >
              <Stethoscope className="w-3.5 h-3.5" />
              <span>{isCheckingDoctor ? t.settings.checking : t.settings.runDoctorBtn}</span>
            </button>

            {doctorReport && (
              <button
                onClick={handleDoctorCleanup}
                disabled={isCleaning}
                className="flex items-center space-x-2 px-3.5 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-750 disabled:opacity-50 text-neutral-300 text-xs font-medium border border-neutral-700 transition-colors"
              >
                <Sparkles className="w-3.5 h-3.5 text-amber-400" />
                <span>{isCleaning ? t.settings.cleaning : t.settings.cleanDoctorBtn}</span>
              </button>
            )}
          </div>

          {cleanMessage && (
            <div className="text-xs text-emerald-400 font-medium bg-emerald-950/40 border border-emerald-500/20 p-2 rounded">
              {cleanMessage}
            </div>
          )}

          {doctorReport && (
            <div className="mt-3 p-3.5 rounded-lg bg-neutral-950 border border-neutral-800 text-xs space-y-2">
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                <div className="p-2.5 rounded bg-neutral-900 border border-neutral-800">
                  <span className="text-neutral-500 block text-[11px] mb-0.5">{t.settings.doctorIntegrity}</span>
                  <span className={`font-medium ${doctorReport.integrity_ok ? 'text-emerald-400' : 'text-rose-400'}`}>
                    {doctorReport.integrity_message}
                  </span>
                </div>
                <div className="p-2.5 rounded bg-neutral-900 border border-neutral-800">
                  <span className="text-neutral-500 block text-[11px] mb-0.5">{t.settings.doctorDbSize}</span>
                  <span className="font-mono text-neutral-200">
                    {formatBytes(doctorReport.database_size_bytes)}
                  </span>
                </div>
                <div className="p-2.5 rounded bg-neutral-900 border border-neutral-800">
                  <span className="text-neutral-500 block text-[11px] mb-0.5">活跃资产 / 总资产</span>
                  <span className="font-mono text-neutral-200">
                    {doctorReport.active_assets} / {doctorReport.total_assets}
                  </span>
                </div>
              </div>

              {doctorReport.missing_assets.length > 0 ? (
                <div className="p-3 rounded bg-rose-950/20 border border-rose-500/30 space-y-1">
                  <div className="flex items-center space-x-1.5 text-rose-400 text-xs font-semibold">
                    <AlertTriangle className="w-3.5 h-3.5" />
                    <span>
                      {t.settings.doctorMissing} ({doctorReport.missing_assets.length})
                    </span>
                  </div>
                  <div className="max-h-24 overflow-y-auto font-mono text-[10px] text-neutral-400 space-y-0.5">
                    {doctorReport.missing_assets.map((p) => (
                      <div key={p} className="truncate">{p}</div>
                    ))}
                  </div>
                </div>
              ) : (
                <div className="text-[11px] text-emerald-400 flex items-center space-x-1 pt-1">
                  <Check className="w-3.5 h-3.5" />
                  <span>{t.settings.doctorMissingNone}</span>
                </div>
              )}
            </div>
          )}
        </div>

        {/* I18N Language Selection Section */}
        <div className="p-5 rounded-xl bg-neutral-900 border border-neutral-800 space-y-4">
          <div className="flex items-center space-x-2 text-sm font-medium text-neutral-200">
            <Globe className="w-4 h-4 text-indigo-400" />
            <span>{t.settings.i18nTitle}</span>
          </div>
          <p className="text-xs text-neutral-400">
            {t.settings.i18nDesc}
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
            {languages.map((lang) => {
              const active = locale === lang.code;
              return (
                <button
                  key={lang.code}
                  onClick={() => setLocale(lang.code)}
                  className={`p-3 rounded-lg border text-left transition-all flex items-center justify-between ${
                    active
                      ? 'bg-indigo-600/10 border-indigo-500/80 text-white shadow-sm shadow-indigo-500/10'
                      : 'bg-neutral-850 border-neutral-700/60 text-neutral-300 hover:border-neutral-600 hover:bg-neutral-800'
                  }`}
                >
                  <div>
                    <div className="text-xs font-semibold">{lang.label}</div>
                    <div className="text-[11px] text-neutral-400 mt-0.5">{lang.desc}</div>
                  </div>
                  {active && (
                    <div className="w-5 h-5 rounded-full bg-indigo-600 flex items-center justify-center text-white">
                      <Check className="w-3 h-3 stroke-[3]" />
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Vault Storage Paths */}
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 space-y-3">
          <div className="flex items-center space-x-2 text-sm font-medium text-neutral-200">
            <HardDrive className="w-4 h-4 text-indigo-400" />
            <span>{t.settings.storageTitle}</span>
          </div>
          <div className="space-y-2 text-xs">
            <div className="flex flex-col space-y-1">
              <span className="text-neutral-500">{t.settings.vaultRootLabel}</span>
              <span className="font-mono text-neutral-300 bg-neutral-800/80 p-2 rounded select-text break-all">
                {vaultInfo?.vault_path || t.settings.notLoaded}
              </span>
            </div>
            <div className="flex flex-col space-y-1">
              <span className="text-neutral-500">{t.settings.databaseFileLabel}</span>
              <span className="font-mono text-neutral-300 bg-neutral-800/80 p-2 rounded select-text break-all">
                {vaultInfo?.database_path || t.settings.notLoaded}
              </span>
            </div>
          </div>
        </div>

        {/* Database & Architecture */}
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 space-y-3">
          <div className="flex items-center space-x-2 text-sm font-medium text-neutral-200">
            <Database className="w-4 h-4 text-emerald-400" />
            <span>{t.settings.engineTitle}</span>
          </div>
          <div className="grid grid-cols-2 gap-3 text-xs">
            <div className="p-3 rounded bg-neutral-850 border border-neutral-800">
              <div className="text-neutral-500 mb-1">{t.settings.dbEngineLabel}</div>
              <div className="text-neutral-200 font-medium">{t.settings.dbEngineVal}</div>
            </div>
            <div className="p-3 rounded bg-neutral-850 border border-neutral-800">
              <div className="text-neutral-500 mb-1">{t.settings.archVersionLabel}</div>
              <div className="text-neutral-200 font-medium">Looma Architecture v{vaultInfo?.version || '0.2'}</div>
            </div>
          </div>
        </div>

        {/* Phase 5: External Interfaces (CLI & MCP) */}
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 space-y-3">
          <div className="flex items-center space-x-2 text-xs font-semibold text-neutral-300">
            <Terminal className="w-4 h-4 text-cyan-400" />
            <span>{t.settings.interfacesTitle}</span>
          </div>
          <p className="text-xs text-neutral-400 leading-relaxed">
            {t.settings.interfacesDesc}
          </p>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs pt-1">
            <div className="p-3 rounded bg-neutral-850 border border-neutral-800">
              <div className="flex items-center space-x-1.5 text-neutral-400 mb-1.5 font-medium">
                <Terminal className="w-3.5 h-3.5 text-neutral-300" />
                <span>{t.settings.cliLabel}</span>
              </div>
              <code className="text-[11px] font-mono text-cyan-300 bg-neutral-900/80 px-2 py-1 rounded block">
                {t.settings.cliCmd}
              </code>
            </div>
            <div className="p-3 rounded bg-neutral-850 border border-neutral-800">
              <div className="flex items-center space-x-1.5 text-neutral-400 mb-1.5 font-medium">
                <Cpu className="w-3.5 h-3.5 text-purple-400" />
                <span>{t.settings.mcpLabel}</span>
              </div>
              <code className="text-[11px] font-mono text-purple-300 bg-neutral-900/80 px-2 py-1 rounded block">
                {t.settings.mcpCmd}
              </code>
            </div>
          </div>
        </div>

        {/* Privacy Baseline */}
        <div className="p-4 rounded-xl bg-neutral-900 border border-neutral-800 space-y-2">
          <div className="flex items-center space-x-2 text-xs font-semibold text-emerald-400">
            <ShieldCheck className="w-4 h-4" />
            <span>{t.settings.privacyTitle}</span>
          </div>
          <p className="text-xs text-neutral-400 leading-relaxed">
            {t.settings.privacyDesc}
          </p>
        </div>
      </div>
    </div>
  );
};

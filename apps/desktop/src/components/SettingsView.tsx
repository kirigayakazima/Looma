import React from 'react';
import { VaultInfo } from '../types';
import { ShieldCheck, HardDrive, Database, Globe, Check } from 'lucide-react';
import { useI18n, Locale } from '../i18n';

interface SettingsViewProps {
  vaultInfo: VaultInfo | null;
}

export const SettingsView: React.FC<SettingsViewProps> = ({ vaultInfo }) => {
  const { locale, setLocale, t } = useI18n();

  const languages: { code: Locale; label: string; desc: string }[] = [
    { code: 'zh-CN', label: t.settings.langZh, desc: '默认语言 (Default)' },
    { code: 'en-US', label: t.settings.langEn, desc: 'English (US)' },
  ];

  return (
    <div className="space-y-6 max-w-4xl">
      <div>
        <h3 className="text-base font-semibold text-neutral-100">{t.settings.title}</h3>
        <p className="text-xs text-neutral-400">{t.settings.subtitle}</p>
      </div>

      <div className="space-y-4">
        {/* I18N Language Selection Section */}
        <div className="p-5 rounded-lg bg-neutral-900 border border-neutral-800 space-y-4">
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

        {/* Vault Paths */}
        <div className="p-4 rounded-lg bg-neutral-900 border border-neutral-800 space-y-3">
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
        <div className="p-4 rounded-lg bg-neutral-900 border border-neutral-800 space-y-3">
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

        {/* Privacy Baseline */}
        <div className="p-4 rounded-lg bg-neutral-900 border border-neutral-800 space-y-2">
          <div className="flex items-center space-x-2 text-sm font-medium text-neutral-200">
            <ShieldCheck className="w-4 h-4 text-indigo-400" />
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

import React, { createContext, useContext, useState, useEffect } from 'react';
import { zhCN, TranslationType } from './locales/zh-CN';
import { enUS } from './locales/en-US';

export type Locale = 'zh-CN' | 'en-US';

interface I18nContextType {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: TranslationType;
}

const translations: Record<Locale, TranslationType> = {
  'zh-CN': zhCN,
  'en-US': enUS,
};

const I18nContext = createContext<I18nContextType | null>(null);

export const I18nProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Default to 'zh-CN' as explicitly requested!
  const [locale, setLocaleState] = useState<Locale>(() => {
    const saved = localStorage.getItem('looma_locale') as Locale | null;
    return saved && (saved === 'zh-CN' || saved === 'en-US') ? saved : 'zh-CN';
  });

  const setLocale = (newLocale: Locale) => {
    setLocaleState(newLocale);
    localStorage.setItem('looma_locale', newLocale);
  };

  useEffect(() => {
    document.documentElement.lang = locale === 'zh-CN' ? 'zh-CN' : 'en';
  }, [locale]);

  const value = {
    locale,
    setLocale,
    t: translations[locale] || zhCN,
  };

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
};

export const useI18n = (): I18nContextType => {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
};

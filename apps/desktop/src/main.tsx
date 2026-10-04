import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { I18nProvider } from './i18n';
import { ThemeProvider } from './theme';
import { initWebMockAdapter } from './webAdapter';
import './index.css';

// Initialize web mock adapter when running in standard browser
initWebMockAdapter();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <I18nProvider>
        <App />
      </I18nProvider>
    </ThemeProvider>
  </React.StrictMode>,
);

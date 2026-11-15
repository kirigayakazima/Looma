import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Bot, RefreshCw } from 'lucide-react';
import { AiServiceStatus } from '../types';

interface AiStatusBadgeProps {
  className?: string;
  onStatusChange?: (status: AiServiceStatus) => void;
}

export const AiStatusBadge: React.FC<AiStatusBadgeProps> = ({
  className = '',
  onStatusChange,
}) => {
  const [status, setStatus] = useState<AiServiceStatus | null>(null);
  const [checking, setChecking] = useState(false);

  const checkStatus = async () => {
    setChecking(true);
    try {
      const res = await invoke<AiServiceStatus>('check_ai_service_status', {
        endpoint: null,
        apiKey: null,
      });
      setStatus(res);
      if (onStatusChange) onStatusChange(res);
    } catch (err: any) {
      console.warn('[AiStatusBadge] check_ai_service_status failed:', err);
      const fallback: AiServiceStatus = {
        online: false,
        endpoint: 'http://127.0.0.1:8081/v1',
        model: 'gemini-flash-lite',
        message: err?.message || 'IPC call failed',
      };
      setStatus(fallback);
      if (onStatusChange) onStatusChange(fallback);
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    checkStatus();
  }, []);

  const isOnline = status?.online ?? false;

  return (
    <div
      onClick={checkStatus}
      className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium border transition-all cursor-pointer select-none ${
        isOnline
          ? 'bg-emerald-950/40 border-emerald-500/40 text-emerald-300 hover:bg-emerald-950/60'
          : 'bg-neutral-900 border-neutral-800 text-neutral-400 hover:bg-neutral-850 hover:text-neutral-300'
      } ${className}`}
      title={
        isOnline
          ? `AI 反代服务在线\n端点: ${status?.endpoint}\n模型: ${status?.model}\n延迟: ${status?.latency_ms ?? 0}ms\n点击刷新状态`
          : `AI 反代服务未连接 (端口 8081)\n端点: ${status?.endpoint || 'http://127.0.0.1:8081/v1'}\n信息: ${status?.message || '无法连接'}\n点击重新检测`
      }
    >
      <span className="relative flex h-2 w-2">
        {isOnline && (
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
        )}
        <span
          className={`relative inline-flex rounded-full h-2 w-2 ${
            isOnline ? 'bg-emerald-500' : 'bg-neutral-600'
          }`}
        />
      </span>

      <Bot className={`w-3.5 h-3.5 ${isOnline ? 'text-emerald-400' : 'text-neutral-500'}`} />

      <span>
        {checking
          ? '检测中...'
          : isOnline
          ? `AI 在线${status?.latency_ms ? ` (${status.latency_ms}ms)` : ''}`
          : 'AI 离线'}
      </span>

      <RefreshCw
        className={`w-3 h-3 text-neutral-500 hover:text-neutral-300 ml-0.5 ${
          checking ? 'animate-spin' : ''
        }`}
      />
    </div>
  );
};

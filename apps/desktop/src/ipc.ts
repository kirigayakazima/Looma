import { invoke } from '@tauri-apps/api/core';

export async function safeInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
  silenceError = false
): Promise<T | null> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err: unknown) {
    const errMsg = (err as { message?: string })?.message || String(err);
    console.warn(`[Tauri IPC] '${cmd}' failed:`, errMsg);
    if (!silenceError && !cmd.startsWith('read_asset_preview') && !cmd.startsWith('get_asset_thumbnail')) {
      window.dispatchEvent(
        new CustomEvent('looma-ipc-error', {
          detail: `操作「${cmd}」返回错误: ${errMsg}`,
        })
      );
    }
    return null;
  }
}

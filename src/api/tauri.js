/**
 * Tauri IPC 安全包装：桌面模式 invoke/listen，预览模式降级。
 */

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * @param {string} cmd
 * @param {Record<string, unknown>} [args]
 */
export async function invokeSafe(cmd, args) {
  if (!isTauri) {
    console.warn(`[preview] invoke skipped: ${cmd}`, args);
    return null;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke(cmd, args);
}

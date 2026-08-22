/**
 * Typed wrappers around the Tauri commands the shell page calls.
 * @module @deepseek-ai/dsh-desktop/tauri-api
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { DshDesktopConfig } from './config.ts'

/** Backend process and navigation state. */
export type BackendState = 'idle' | 'installing' | 'starting' | 'ready' | 'updating' | 'error'

/** Snapshot the Rust backend emits and returns from get_status. */
export interface BackendStatus {
  state: BackendState
  url?: string
  message?: string
  config: DshDesktopConfig
  installedVersion?: string
  latestVersion?: string
  canLaunchLocal: boolean
  platform: string
}

/**
 * Load the persisted config plus live backend status.
 * @returns the current snapshot.
 */
export async function getStatus(): Promise<BackendStatus> {
  return invoke<BackendStatus>('get_status')
}

/**
 * Persist a validated config. Does not start or stop the backend.
 * @param config - normalized settings.
 * @returns the stored config.
 */
export async function setConfig(config: DshDesktopConfig): Promise<DshDesktopConfig> {
  return invoke<DshDesktopConfig>('set_config', { config })
}

/**
 * Start or connect according to the stored launch mode.
 * @returns the status after the command is accepted.
 */
export async function startBackend(): Promise<BackendStatus> {
  return invoke<BackendStatus>('start_dsh')
}

/**
 * Stop a locally spawned `dsh web` process. Connect mode is a no-op.
 * @returns the status after stop.
 */
export async function stopBackend(): Promise<BackendStatus> {
  return invoke<BackendStatus>('stop_dsh')
}

/**
 * Navigate the current WebView to the ready DSH URL.
 */
export async function openWeb(): Promise<void> {
  await invoke('open_web')
}

/**
 * Navigate the current WebView back to the shell settings page.
 */
export async function openSettings(): Promise<void> {
  await invoke('open_settings')
}

/**
 * Compare the managed DSH install with the npm registry.
 * @returns current and latest versions when they can be read.
 */
export async function checkDshUpdates(): Promise<{ installed?: string; latest?: string }> {
  return invoke('check_dsh_updates')
}

/**
 * Install or upgrade `@deepseek-ai/dsh` in the app-data runtime prefix.
 */
export async function updateDsh(): Promise<BackendStatus> {
  return invoke<BackendStatus>('update_dsh')
}

/**
 * Subscribe to live backend status events.
 * @param handler - called with each snapshot.
 * @returns an unlisten function.
 */
export async function onStatus(handler: (status: BackendStatus) => void): Promise<UnlistenFn> {
  return listen<BackendStatus>('dsh-status', (event) => { handler(event.payload) })
}

/**
 * Subscribe to installer/updater stdout lines.
 * @param handler - called with each line.
 * @returns an unlisten function.
 */
export async function onUpdateProgress(handler: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>('dsh-update-progress', (event) => { handler(event.payload) })
}

/**
 * Subscribe to local `dsh web` spawn stdout/stderr.
 * @param handler - called with each line.
 * @returns an unlisten function.
 */
export async function onSpawnLog(handler: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>('dsh-spawn-log', (event) => { handler(event.payload) })
}

/**
 * Subscribe to the tray/settings request that keeps the shell page in place.
 */
export async function onOpenSettings(handler: () => void): Promise<UnlistenFn> {
  return listen('dsh-open-settings', () => { handler() })
}

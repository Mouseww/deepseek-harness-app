/**
 * Shared host/port and launch-mode validation for the Tauri shell.
 * The Rust backend persists the same field names; this module is the
 * browser-side check that rejects a form before invoke.
 * @module @deepseek-ai/dsh-desktop/config
 */

/** How the shell reaches DSH web. */
export type LaunchMode = 'local' | 'connect'

/** Persisted shell settings. */
export interface DshDesktopConfig {
  /** Bind or connect host. Local launch currently requires `127.0.0.1`. */
  host: string
  /** Listen or connect port. `0` asks the OS for a free port on local launch. */
  port: number
  /** Start or connect automatically when the window opens. */
  autoStart: boolean
  /** Spawn a local `dsh web` process, or only navigate to an existing server. */
  launchMode: LaunchMode
}

/** Factory defaults used when no store file exists. */
export const DEFAULT_CONFIG: DshDesktopConfig = {
  host: '127.0.0.1',
  port: 3080,
  autoStart: true,
  launchMode: 'local',
}

/** Why a candidate config must not be saved or used to launch. */
export class ConfigError extends Error {
  override readonly name = 'ConfigError'
}

/**
 * Parse a port field. Local launch accepts `0` (OS-assigned); connect mode does not.
 * @param raw - form text or a number already parsed by the backend.
 * @param launchMode - which launch mode the port will serve.
 * @returns the integer port.
 */
export function parsePort(raw: string | number, launchMode: LaunchMode): number {
  const value = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isInteger(value) || value < 0 || value > 65_535) {
    throw new ConfigError('port must be an integer from 0 to 65535')
  }
  if (launchMode === 'connect' && value === 0) {
    throw new ConfigError('connect mode needs an explicit nonzero port')
  }
  return value
}

/**
 * Normalize and reject a host string.
 * Local launch cannot bind `0.0.0.0` because `dsh web` refuses that bind.
 * @param raw - form text.
 * @param launchMode - which launch mode the host will serve.
 * @returns the trimmed host.
 */
export function parseHost(raw: string, launchMode: LaunchMode): string {
  const host = raw.trim()
  if (host === '') throw new ConfigError('host must not be empty')
  if (/\s/u.test(host)) throw new ConfigError('host must not contain whitespace')
  if (host.includes('/') || host.includes(':')) {
    throw new ConfigError('host must be a hostname or IPv4 literal, without a scheme or port')
  }
  if (launchMode === 'local' && host === '0.0.0.0') {
    throw new ConfigError(
      'host 0.0.0.0 is not supported: dsh web refuses all-interfaces binds; use 127.0.0.1',
    )
  }
  return host
}

/**
 * Validate a complete config object.
 * @param input - candidate settings, possibly partial from a form.
 * @returns a normalized config.
 */
export function parseConfig(input: {
  host: string
  port: string | number
  autoStart: boolean
  launchMode: string
}): DshDesktopConfig {
  if (input.launchMode !== 'local' && input.launchMode !== 'connect') {
    throw new ConfigError('launchMode must be local or connect')
  }
  return {
    host: parseHost(input.host, input.launchMode),
    port: parsePort(input.port, input.launchMode),
    autoStart: input.autoStart,
    launchMode: input.launchMode,
  }
}

/**
 * Build the loopback or remote URL the WebView should load.
 * @param config - a validated config.
 * @returns an `http://host:port/` URL. Port `0` is not a navigable URL.
 */
export function webUrl(config: Pick<DshDesktopConfig, 'host' | 'port'>): string {
  if (config.port === 0) {
    throw new ConfigError('cannot build a URL until dsh web reports its assigned port')
  }
  return `http://${config.host}:${String(config.port)}/`
}

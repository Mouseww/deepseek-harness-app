/**
 * Parse the `dsh web` readiness line that supervisors wait for.
 * @module @deepseek-ai/dsh-desktop/ready
 */

/** Captured listen URL from a ready line. */
export interface ReadyUrl {
  /** Canonical loopback URL printed first, e.g. `http://127.0.0.1:3080`. */
  url: string
  /** Optional LAN URL from the parenthetical, when the server bound all interfaces. */
  lanUrl?: string
}

const READY = /^dsh web: (?<url>https?:\/\/\S+?)(?: \(LAN: (?<lanUrl>https?:\/\/\S+)\))?\s*$/u

/**
 * Extract the listen URL from one stdout line.
 * @param line - a single line of `dsh web` output, without a trailing newline.
 * @returns the parsed URLs, or `undefined` when the line is not the ready signal.
 */
export function parseReadyLine(line: string): ReadyUrl | undefined {
  const groups = READY.exec(line.trimEnd())?.groups
  if (groups === undefined) return undefined
  const url = groups.url
  if (url === undefined || url === '') return undefined
  const lanUrl = groups.lanUrl
  if (typeof lanUrl === 'string' && lanUrl !== '') return { url, lanUrl }
  return { url }
}

/**
 * Build the argv suffix `dsh web` receives after the launcher token sequence.
 * @param host - bind host.
 * @param port - bind port, including `0`.
 * @returns `['--host', host, '--port', port]`.
 */
export function webLaunchArgs(host: string, port: number): string[] {
  return ['--host', host, '--port', String(port)]
}

import { describe, expect, it, vi } from 'vitest'
import authBridgeScript from '/src-tauri/src/auth_bridge.js?raw'

type Listener = (event: { reason?: unknown; target?: unknown }) => void

function response(status: number): Response {
  return { status } as Response
}

function installBridge(statuses: number[], pathname = '/') {
  const listeners = new Map<string, Listener>()
  const replace = vi.fn()
  const nativeFetch = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) =>
    response(statuses.shift() ?? 200),
  )
  const windowMock = {
    location: { origin: 'http://127.0.0.1:3080', pathname, replace },
    fetch: nativeFetch,
    addEventListener(type: string, listener: Listener) {
      listeners.set(type, listener)
    },
  }
  const run = new Function('window', 'URL', 'Request', authBridgeScript)
  run(windowMock, URL, Request)
  return { windowMock, nativeFetch, listeners, replace }
}

describe('DSH authentication recovery bridge', () => {
  it('redirects a protected same-origin fetch to login on 401', async () => {
    const { windowMock, replace } = installBridge([401])

    await windowMock.fetch('/api/agentPreset.list')

    expect(replace).toHaveBeenCalledWith('/login')
  })

  it('does not redirect cross-origin requests or loop on the login page', async () => {
    const crossOrigin = installBridge([401])
    await crossOrigin.windowMock.fetch('https://example.com/api')
    expect(crossOrigin.replace).not.toHaveBeenCalled()

    const login = installBridge([401], '/login')
    await login.windowMock.fetch('/auth/login')
    expect(login.replace).not.toHaveBeenCalled()
  })

  it('probes authentication after a dynamic import Failed to fetch', async () => {
    const { listeners, replace } = installBridge([401])

    listeners.get('unhandledrejection')?.({ reason: new TypeError('Failed to fetch dynamically imported module') })
    await Promise.resolve()
    await Promise.resolve()

    expect(replace).toHaveBeenCalledWith('/login')
  })
})

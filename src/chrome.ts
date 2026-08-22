/**
 * macOS-style traffic-light chrome for the undecorated Tauri window.
 * Theme tokens come from the DSH webview via dsh-theme.
 */
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'

export interface TitlebarTheme {
  bg: string
  fg: string
}

/**
 * Parse an rgb/rgba/hex color into sRGB luminance, or null when unknown.
 */
export function luminanceOf(color: string): number | null {
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/iu.exec(color.trim())
  if (hex !== null) {
    let raw = hex[1] ?? ''
    if (raw.length === 3) raw = [...raw].map((ch) => ch + ch).join('')
    const n = Number.parseInt(raw, 16)
    return luminance((n >> 16) & 255, (n >> 8) & 255, n & 255)
  }
  const rgb = /rgba?\(\s*([\d.]+)\s*[,\s]\s*([\d.]+)\s*[,\s]\s*([\d.]+)/iu.exec(color)
  if (rgb === null) return null
  return luminance(Number(rgb[1]), Number(rgb[2]), Number(rgb[3]))
}

function luminance(r: number, g: number, b: number): number {
  const lin = (channel: number) => {
    const c = channel / 255
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/**
 * Paint titlebar CSS variables from a sampled page color.
 */
export function applyTitlebarTheme(theme: TitlebarTheme): void {
  const root = document.documentElement
  root.style.setProperty('--tb-bg', theme.bg)
  root.style.setProperty('--tb-fg', theme.fg)
  const lum = luminanceOf(theme.bg)
  document.body.classList.toggle('chrome-light', lum !== null && lum > 0.55)
}

/**
 * Wire close / minimize / zoom and live theme following.
 */
export function disableContextMenu(): void {
  const block = (event: Event) => {
    event.preventDefault()
    event.stopImmediatePropagation()
  }
  window.addEventListener('contextmenu', block, true)
  document.addEventListener('contextmenu', block, true)
  const onAux = (event: MouseEvent) => {
    if (event.button === 2) block(event)
  }
  window.addEventListener('auxclick', onAux, true)
  window.addEventListener('mouseup', onAux, true)
}

export async function attachChrome(): Promise<void> {
  disableContextMenu()
  const win = getCurrentWindow()
  const close = required('win-close')
  const min = required('win-min')
  const zoom = required('win-zoom')

  close.addEventListener('click', (event) => {
    event.stopPropagation()
    void win.close()
  })
  min.addEventListener('click', (event) => {
    event.stopPropagation()
    void win.minimize()
  })
  zoom.addEventListener('click', (event) => {
    event.stopPropagation()
    void win.toggleMaximize()
  })

  const syncMaximized = async () => {
    document.body.classList.toggle('window-maximized', await win.isMaximized())
  }
  await syncMaximized()
  await win.onResized(() => { void syncMaximized() })
  await win.onFocusChanged(({ payload: focused }) => {
    document.body.classList.toggle('window-inactive', !focused)
  })
  document.body.classList.toggle('window-inactive', !(await win.isFocused()))

  await listen<TitlebarTheme>('dsh-theme', (event) => {
    applyTitlebarTheme(event.payload)
  })
}

function required(id: string): HTMLElement {
  const node = document.getElementById(id)
  if (node === null) throw new Error(`desktop shell: missing #${id}`)
  return node
}

/**
 * Settings page: persist host/port, start or connect to dsh web, and update DSH.
 * After the backend reports ready a child webview loads the official Web UI
 * under the macOS-style titlebar.
 */
import { attachChrome } from './chrome.ts'
import { parseConfig, type DshDesktopConfig } from './config.ts'
import {
  checkAppUpdate,
  checkDshUpdates,
  getAppUpdate,
  getStatus,
  installAppUpdate,
  onAppUpdate,
  onOpenSettings,
  onSpawnLog,
  onStatus,
  onUpdateProgress,
  openWeb,
  setConfig,
  startBackend,
  stopBackend,
  updateDsh,
  type AppUpdateStatus,
  type BackendStatus,
} from './tauri-api.ts'

const hostInput = requiredElement('host') as HTMLInputElement
const portInput = requiredElement('port') as HTMLInputElement
const modeInput = requiredElement('launch-mode') as HTMLSelectElement
const autoStartInput = requiredElement('auto-start') as HTMLInputElement
const form = requiredElement('settings') as HTMLFormElement
const stopButton = requiredElement('stop') as HTMLButtonElement
const openButton = requiredElement('open-web') as HTMLButtonElement
const checkButton = requiredElement('check-updates') as HTMLButtonElement
const updateButton = requiredElement('update-dsh') as HTMLButtonElement
const lamp = requiredElement('lamp')
const statusLabel = requiredElement('status-label')
const statusDetail = requiredElement('status-detail')
const urlLine = requiredElement('url-line')
const versionLine = requiredElement('version-line')
const logEl = requiredElement('log')
const localHint = requiredElement('local-hint')
const saveStart = requiredElement('save-start') as HTMLButtonElement
const settingsStack = requiredElement('settings-stack')
const bootRail = requiredElement('boot-rail')
const splash = requiredElement('splash')
const updateChip = requiredElement('update-chip') as HTMLButtonElement
const appVersionLine = requiredElement('app-version-line')
const checkAppButton = requiredElement('check-app-update') as HTMLButtonElement
const installAppButton = requiredElement('install-app-update') as HTMLButtonElement
const appUpdateHint = requiredElement('app-update-hint')
const settingsRequested = new URLSearchParams(location.search).has('settings')
let forceSettings = settingsRequested

const LABELS: Record<BackendStatus['state'], string> = {
  idle: 'Idle',
  installing: 'Installing DSH',
  starting: 'Starting',
  ready: 'Ready',
  updating: 'Updating DSH',
  error: 'Error',
}

/**
 * Look up a required element.
 * @param id - DOM id.
 * @returns the element.
 */
function requiredElement(id: string): HTMLElement {
  const node = document.getElementById(id)
  if (node === null) throw new Error(`desktop shell: missing #${id}`)
  return node
}

/**
 * Read the form into a validated config.
 * @returns the normalized config.
 */
function readForm(): DshDesktopConfig {
  return parseConfig({
    host: hostInput.value,
    port: portInput.value,
    autoStart: autoStartInput.checked,
    launchMode: modeInput.value,
  })
}

/**
 * Paint the form from a stored config.
 * @param config - persisted settings.
 */
function writeForm(config: DshDesktopConfig): void {
  hostInput.value = config.host
  portInput.value = String(config.port)
  modeInput.value = config.launchMode
  autoStartInput.checked = config.autoStart
}

/**
 * Paint status chrome from a backend snapshot.
 * @param status - latest snapshot.
 */
function render(status: BackendStatus): void {
  lamp.dataset['state'] = status.state
  statusLabel.textContent = LABELS[status.state]
  statusDetail.textContent = status.message ?? defaultMessage(status)
  if (status.url !== undefined) {
    urlLine.hidden = false
    urlLine.textContent = status.url
  } else {
    urlLine.hidden = true
    urlLine.textContent = ''
  }
  versionLine.textContent = `Installed ${status.installedVersion ?? '—'} · Latest ${status.latestVersion ?? '—'}`
  openButton.hidden = status.state !== 'ready'
  const busy = status.state === 'starting' || status.state === 'installing' || status.state === 'updating'
  saveStart.disabled = busy
  stopButton.disabled = busy || status.state === 'idle'
  updateButton.disabled = busy
  if (!status.canLaunchLocal) {
    localHint.textContent = 'This platform cannot spawn Node. Use connect mode and enter a reachable dsh web host and port.'
    for (const option of modeInput.options) {
      if (option.value === 'local') option.disabled = true
    }
  }
  const showSettings = forceSettings || status.state === 'error'
  settingsStack.hidden = !showSettings
  splash.hidden = showSettings && status.state === 'ready'
  paintBootRail(status)
}

/**
 * Highlight the current first-launch stage on the splash rail.
 */
function paintBootRail(status: BackendStatus): void {
  const step = bootStep(status)
  const order = ['runtime', 'plugins', 'web', 'ready']
  const active = order.indexOf(step)
  for (const node of bootRail.querySelectorAll('li')) {
    const key = node.getAttribute('data-step') ?? ''
    const index = order.indexOf(key)
    node.setAttribute('data-active', index === active ? '1' : '0')
    node.setAttribute('data-done', index >= 0 && index < active ? '1' : '0')
  }
}

/**
 * Map backend state onto a named boot stage.
 */
function bootStep(status: BackendStatus): string {
  if (status.state === 'ready') return 'ready'
  if (status.state === 'error' || status.state === 'idle') return 'runtime'
  if (status.state === 'starting') return 'web'
  const message = (status.message ?? '').toLowerCase()
  if (message.includes('plugin')) return 'plugins'
  return 'runtime'
}

/**
 * Fallback status copy when the backend omitted a message.
 * @param status - latest snapshot.
 * @returns a short sentence.
 */
function defaultMessage(status: BackendStatus): string {
  switch (status.state) {
    case 'idle': return 'Configure host and port, then start.'
    case 'installing': return 'Installing @deepseek-ai/dsh into the app runtime prefix.'
    case 'starting': return status.config.launchMode === 'connect'
      ? 'Waiting for the configured host.'
      : 'Spawning dsh web and waiting for the ready line.'
    case 'ready': return 'Official Web UI is reachable.'
    case 'updating': return 'Updating the managed DSH install.'
    case 'error': return 'See the log below.'
    default: {
      const _exhaustive: never = status.state
      return _exhaustive
    }
  }
}

/**
 * Paint desktop-app update chrome from a GitHub release snapshot.
 */
function paintAppUpdate(status: AppUpdateStatus): void {
  const latest = status.latest ?? '—'
  appVersionLine.textContent = `This app ${status.current} · Latest ${latest}`
  appUpdateHint.textContent = status.message ?? 'Startup checks GitHub Releases and can download the matching installer so you do not have to fetch it by hand.'
  const busy = status.state === 'checking' || status.state === 'downloading' || status.state === 'installing'
  checkAppButton.disabled = busy
  installAppButton.hidden = !status.available && status.state !== 'downloading' && status.state !== 'installing'
  installAppButton.disabled = busy
  if (status.state === 'downloading') {
    const total = status.bytesTotal
    const pct = total && total > 0 ? Math.round((status.bytesDownloaded / total) * 100) : 0
    installAppButton.textContent = total ? `Downloading ${pct}%` : 'Downloading…'
    updateChip.hidden = false
    updateChip.dataset['busy'] = '1'
    updateChip.textContent = total ? `${pct}%` : '…'
  } else if (status.available) {
    installAppButton.textContent = `Download & install ${status.latest ?? ''}`.trim()
    updateChip.hidden = false
    updateChip.dataset['busy'] = '0'
    updateChip.textContent = `Update ${status.latest ?? ''}`.trim()
  } else {
    installAppButton.textContent = 'Download & install'
    updateChip.hidden = true
    updateChip.dataset['busy'] = '0'
    updateChip.textContent = 'Update'
  }
}

/**
 * Append one updater line to the log panel.
 * @param line - a stdout/stderr line.
 */
function appendLog(line: string): void {
  logEl.hidden = false
  logEl.textContent = logEl.textContent === '' ? line : `${logEl.textContent}\n${line}`
  logEl.scrollTop = logEl.scrollHeight
}

/**
 * Surface a user-facing failure on the status card.
 * @param error - thrown value from invoke or validation.
 */
function showError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error)
  statusLabel.textContent = LABELS.error
  statusDetail.textContent = message
  lamp.dataset['state'] = 'error'
}

form.addEventListener('submit', (event) => {
  event.preventDefault()
  void (async () => {
    try {
      const config = readForm()
      await setConfig(config)
      render(await startBackend())
    } catch (error) {
      showError(error)
    }
  })()
})

stopButton.addEventListener('click', () => {
  void stopBackend().then(render).catch(showError)
})

openButton.addEventListener('click', () => {
  forceSettings = false
  settingsStack.hidden = true
  void openWeb().catch(showError)
})

checkButton.addEventListener('click', () => {
  void checkDshUpdates()
    .then(async () => { render(await getStatus()) })
    .catch(showError)
})

updateButton.addEventListener('click', () => {
  logEl.textContent = ''
  void updateDsh().then(render).catch(showError)
})

checkAppButton.addEventListener('click', () => {
  void checkAppUpdate().then(paintAppUpdate).catch(showError)
})

installAppButton.addEventListener('click', () => {
  void installAppUpdate().then(paintAppUpdate).catch(showError)
})

updateChip.addEventListener('click', (event) => {
  event.stopPropagation()
  if (updateChip.dataset['busy'] === '1') return
  void installAppUpdate().then(paintAppUpdate).catch(showError)
})

void (async () => {
  try {
    await attachChrome()
  } catch {
    document.body.classList.add('browser')
  }
  await onOpenSettings(() => {
    forceSettings = true
    settingsStack.hidden = false
    splash.hidden = true
  })
  const status = await getStatus()
  writeForm(status.config)
  render(status)
  await onStatus((next) => { render(next) })
  await onUpdateProgress(appendLog)
  await onSpawnLog(appendLog)
  await onAppUpdate(paintAppUpdate)
  try {
    paintAppUpdate(await getAppUpdate())
  } catch {
    // GitHub check is optional on first paint.
  }
})().catch(showError)

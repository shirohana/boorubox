// Popup: plain TypeScript over the DOM (design D1). It draws three things —
// the connection line, the history and the port field — and asks the worker to
// do anything that touches the network.

import { probeStatus } from '../delivery/client.js'
import { clearDelivered, discardEntry, ENTRY_PREFIX, readHistory } from '../history/store.js'
import { getPort, isValidPort, setPort, statusEndpoint } from '../settings.js'
import { RETRY_ALL, RETRY_CAPTURE } from '../messages.js'
import { POLL_INTERVAL_MS, renderConnection } from './connection.js'
import { DISCARD_ACTION, renderHistory, RETRY_ACTION } from './history-view.js'

export function mountPopup(root: ParentNode): () => void {
  const connection = root.querySelector('#connection')!
  const history = root.querySelector('#history')!
  const retryAll = root.querySelector<HTMLButtonElement>('#retry-all')!
  const clear = root.querySelector<HTMLButtonElement>('#clear')!
  const port = root.querySelector<HTMLInputElement>('#port')!
  const portError = root.querySelector<HTMLElement>('#port-error')!

  async function drawConnection() {
    connection.replaceChildren(renderConnection(await probeStatus(statusEndpoint(await getPort()))))
  }

  async function drawHistory() {
    const entries = await readHistory()
    history.replaceChildren(renderHistory(entries))
    retryAll.hidden = !entries.some((entry) => entry.status === 'failed' && entry.hasBytes)
    clear.hidden = !entries.some((entry) => entry.status === 'delivered')
  }

  history.addEventListener('click', (event) => {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>('button[data-action]')
    if (!button?.dataset.id) {
      return
    }
    if (button.dataset.action === RETRY_ACTION) {
      void chrome.runtime.sendMessage({ type: RETRY_CAPTURE, id: button.dataset.id })
    }
    if (button.dataset.action === DISCARD_ACTION) {
      void discardEntry(button.dataset.id).then(drawHistory)
    }
  })

  retryAll.addEventListener('click', () => {
    void chrome.runtime.sendMessage({ type: RETRY_ALL })
  })

  clear.addEventListener('click', () => {
    void clearDelivered().then(drawHistory)
  })

  port.addEventListener('change', () => {
    const value = Number(port.value)
    portError.hidden = isValidPort(value)
    if (!isValidPort(value)) {
      portError.textContent = 'Enter a port between 1 and 65535'
      return
    }
    void setPort(value).then(drawConnection)
  })

  // The worker writes every entry change to storage, so the list follows a
  // retry that finishes while the popup is open without being told (design D2).
  const onStorage = (changes: Record<string, unknown>) => {
    if (Object.keys(changes).some((key) => key.startsWith(ENTRY_PREFIX))) {
      void drawHistory()
    }
  }
  chrome.storage.onChanged.addListener(onStorage)

  void getPort().then((current) => {
    port.value = String(current)
  })
  void drawConnection()
  void drawHistory()

  // Polled only while the popup is open: an MV3 worker has no reliable timer,
  // so a background poll would mean an alarm every minute for ever to keep a
  // dot green nobody is looking at (design D7).
  const timer = setInterval(() => void drawConnection(), POLL_INTERVAL_MS)
  return () => {
    clearInterval(timer)
    chrome.storage.onChanged.removeListener(onStorage)
  }
}

if (typeof document !== 'undefined' && document.querySelector('#connection')) {
  mountPopup(document)
}

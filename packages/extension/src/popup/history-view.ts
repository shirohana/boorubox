// The history list. A pure function from entries to DOM: the popup redraws the
// whole list whenever storage changes, which for a few hundred rows is cheaper
// to get right than a diff.

import type { HistoryEntry } from '../history/store.js'

export const RETRY_ACTION = 'retry'
export const DISCARD_ACTION = 'discard'

export function renderHistory(entries: HistoryEntry[]): HTMLElement {
  const list = document.createElement('ul')
  list.className = 'history'

  if (entries.length === 0) {
    const empty = document.createElement('li')
    empty.className = 'empty'
    empty.textContent = 'No captures yet. Right-click an image and choose “Save to BooruBox”.'
    list.append(empty)
    return list
  }

  for (const entry of entries) {
    list.append(renderRow(entry))
  }
  return list
}

function renderRow(entry: HistoryEntry): HTMLElement {
  const row = document.createElement('li')
  row.className = `row row--${entry.status}`
  row.dataset.id = entry.id
  row.dataset.status = entry.status

  row.append(thumbnail(entry), details(entry), actions(entry))
  return row
}

function thumbnail(entry: HistoryEntry): HTMLElement {
  const image = document.createElement('img')
  image.className = 'thumb'
  image.alt = ''
  if (entry.thumbnail) {
    image.src = entry.thumbnail
  }
  return image
}

function details(entry: HistoryEntry): HTMLElement {
  const details = document.createElement('div')
  details.className = 'details'

  const title = document.createElement('p')
  title.className = 'title'
  title.textContent = entry.pageTitle || hostOf(entry.pageUrl)
  title.title = entry.pageUrl

  const source = document.createElement('p')
  source.className = 'source'
  source.textContent = entry.imageUrl
  source.title = entry.imageUrl

  const meta = document.createElement('p')
  meta.className = 'meta'
  meta.textContent = [
    formatTime(entry.capturedAt),
    formatSize(entry.size),
    statusText(entry),
  ].join(' · ')

  details.append(title, source, meta)
  return details
}

function actions(entry: HistoryEntry): HTMLElement {
  const actions = document.createElement('div')
  actions.className = 'actions'
  if (entry.status !== 'failed') {
    return actions
  }

  // No Retry without bytes: one that re-fetched would silently save different
  // bytes than the ones captured (design D5).
  if (entry.hasBytes) {
    actions.append(button('Retry', RETRY_ACTION, entry.id))
  }
  actions.append(button('Discard', DISCARD_ACTION, entry.id))
  return actions
}

function button(label: string, action: string, id: string): HTMLButtonElement {
  const button = document.createElement('button')
  button.type = 'button'
  button.textContent = label
  button.dataset.action = action
  button.dataset.id = id
  return button
}

function statusText(entry: HistoryEntry): string {
  if (entry.status === 'delivered') return 'Saved'
  if (entry.status === 'pending') return 'Sending…'
  if (!entry.hasBytes) return `Failed · ${entry.reason ?? 'the bytes are gone'} · discard only`
  return `Failed · ${entry.reason ?? 'unknown reason'}`
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function formatTime(epochMs: number): string {
  return new Date(epochMs).toLocaleString()
}

function hostOf(url: string): string {
  try {
    return new URL(url).host
  } catch {
    return url
  }
}

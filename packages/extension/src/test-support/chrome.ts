// A `chrome` global for tests. Only the surface the extension actually calls is
// modelled, and every call lands in `events` in the order it was made, so a
// test can assert that a rule was in place *before* the fetch that needed it.

import { vi } from 'vitest'

type Mock = ReturnType<typeof vi.fn>

export interface FakeChrome {
  /** Every modelled call, in order, as `verb:detail`. */
  events: string[]
  tabs: { sendMessage: Mock }
  runtime: { getURL: Mock, sendMessage: Mock, lastError: undefined }
  contextMenus: { create: Mock, onClicked: FakeEvent }
  notifications: { create: Mock, clear: Mock, onClicked: FakeEvent }
  action: { setBadgeText: Mock, setBadgeBackgroundColor: Mock, openPopup: Mock }
  storage: {
    local: FakeStorageArea
    onChanged: FakeEvent
  }
  declarativeNetRequest: {
    updateDynamicRules: Mock
    RuleActionType: { MODIFY_HEADERS: string }
    HeaderOperation: { SET: string }
    ResourceType: { XMLHTTPREQUEST: string }
    rules: Map<number, unknown>
  }
  /** What the badge currently reads. */
  badgeText: () => string
}

export interface FakeEvent {
  addListener: (listener: (...args: never[]) => unknown) => void
  removeListener: (listener: (...args: never[]) => unknown) => void
  emit: (...args: unknown[]) => void
  listeners: ((...args: never[]) => unknown)[]
}

export interface FakeStorageArea {
  data: Map<string, unknown>
  get: Mock
  set: Mock
  remove: Mock
  clear: Mock
}

function fakeEvent(): FakeEvent {
  const listeners: ((...args: never[]) => unknown)[] = []
  return {
    listeners,
    addListener: (listener) => {
      listeners.push(listener)
    },
    removeListener: (listener) => {
      const at = listeners.indexOf(listener)
      if (at !== -1) listeners.splice(at, 1)
    },
    emit: (...args) => {
      for (const listener of listeners) {
        (listener as (...a: unknown[]) => unknown)(...args)
      }
    },
  }
}

/** `chrome.storage.local`, including the `onChanged` the popup redraws on. */
function fakeStorage(onChanged: FakeEvent): FakeStorageArea {
  const data = new Map<string, unknown>()

  const announce = (changes: Record<string, { oldValue?: unknown, newValue?: unknown }>) => {
    if (Object.keys(changes).length > 0) onChanged.emit(changes, 'local')
  }

  return {
    data,
    get: vi.fn(async (keys?: string | string[] | Record<string, unknown> | null) => {
      if (keys == null) return Object.fromEntries(data)
      const wanted = typeof keys === 'string'
        ? [keys]
        : Array.isArray(keys) ? keys : Object.keys(keys)
      const out: Record<string, unknown> = {}
      for (const key of wanted) {
        if (data.has(key)) out[key] = data.get(key)
        else if (!Array.isArray(keys) && typeof keys === 'object') out[key] = keys[key]
      }
      return out
    }),
    set: vi.fn(async (items: Record<string, unknown>) => {
      const changes: Record<string, { oldValue?: unknown, newValue?: unknown }> = {}
      for (const [key, value] of Object.entries(items)) {
        changes[key] = { oldValue: data.get(key), newValue: value }
        data.set(key, value)
      }
      announce(changes)
    }),
    remove: vi.fn(async (keys: string | string[]) => {
      const changes: Record<string, { oldValue?: unknown }> = {}
      for (const key of Array.isArray(keys) ? keys : [keys]) {
        if (data.has(key)) {
          changes[key] = { oldValue: data.get(key) }
          data.delete(key)
        }
      }
      announce(changes)
    }),
    clear: vi.fn(async () => {
      data.clear()
    }),
  }
}

export function makeFakeChrome(): FakeChrome {
  const events: string[] = []
  const rules = new Map<number, unknown>()
  const storageChanged = fakeEvent()
  let badge = ''

  return {
    events,
    badgeText: () => badge,
    tabs: {
      sendMessage: vi.fn(async () => {
        events.push('tabs.sendMessage')
        return undefined
      }),
    },
    runtime: {
      getURL: vi.fn((path: string) => `chrome-extension://fake/${path}`),
      sendMessage: vi.fn(async () => undefined),
      lastError: undefined,
    },
    contextMenus: {
      create: vi.fn(),
      onClicked: fakeEvent(),
    },
    notifications: {
      create: vi.fn(async () => {
        events.push('notify')
        return 'notification-id'
      }),
      clear: vi.fn(async () => true),
      onClicked: fakeEvent(),
    },
    action: {
      setBadgeText: vi.fn(async ({ text }: { text: string }) => {
        badge = text
        events.push(`badge:${text}`)
      }),
      setBadgeBackgroundColor: vi.fn(async () => {}),
      openPopup: vi.fn(async () => {}),
    },
    storage: {
      local: fakeStorage(storageChanged),
      onChanged: storageChanged,
    },
    declarativeNetRequest: {
      rules,
      RuleActionType: { MODIFY_HEADERS: 'modifyHeaders' },
      HeaderOperation: { SET: 'set' },
      ResourceType: { XMLHTTPREQUEST: 'xmlhttprequest' },
      updateDynamicRules: vi.fn(async (options: {
        addRules?: { id: number }[]
        removeRuleIds?: number[]
      }) => {
        // Chrome applies the removals first; a test that asserts ordering only
        // means anything if the fake agrees.
        for (const id of options.removeRuleIds ?? []) {
          if (rules.delete(id)) events.push(`rule.remove:${id}`)
        }
        for (const rule of options.addRules ?? []) {
          if (rules.has(rule.id)) {
            throw new Error(`a dynamic rule with id ${rule.id} already exists`)
          }
          rules.set(rule.id, rule)
          events.push(`rule.add:${rule.id}`)
        }
      }),
    },
  }
}

/** Install the fake as the `chrome` global for one test. */
export function installFakeChrome(): FakeChrome {
  const fake = makeFakeChrome()
  vi.stubGlobal('chrome', fake)
  return fake
}

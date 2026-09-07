// The one setting the extension has: which port the app listens on (design D6).
// `DEFAULT_PORT` is imported rather than retyped — §5 fixes that number in one
// place, and a second copy here is a second thing to keep in step.

import { DEFAULT_PORT } from '@boorubox/shared'

export const PORT_KEY = 'port'

/** The app binds a loopback port and nothing else, so the host is not a setting. */
const HOST = '127.0.0.1'

export const MIN_PORT = 1
export const MAX_PORT = 65535

export async function getPort(): Promise<number> {
  const stored = await chrome.storage.local.get(PORT_KEY)
  const port = stored[PORT_KEY]
  return isValidPort(port) ? port : DEFAULT_PORT
}

/** Raises on a value the app could never be listening on. */
export async function setPort(port: number): Promise<void> {
  if (!isValidPort(port)) {
    throw new Error(`${port} is not a port between ${MIN_PORT} and ${MAX_PORT}`)
  }
  await chrome.storage.local.set({ [PORT_KEY]: port })
}

export function isValidPort(port: unknown): port is number {
  return typeof port === 'number'
    && Number.isInteger(port)
    && port >= MIN_PORT
    && port <= MAX_PORT
}

export function capturesEndpoint(port: number): string {
  return `http://${HOST}:${port}/captures`
}

export function statusEndpoint(port: number): string {
  return `http://${HOST}:${port}/status`
}

/**
 * Where a coming capture is announced, and — given its id — where that
 * announcement is withdrawn (design D2).
 */
export function pendingEndpoint(port: number, id?: string): string {
  const announcements = `${capturesEndpoint(port)}/pending`
  return id === undefined ? announcements : `${announcements}/${encodeURIComponent(id)}`
}

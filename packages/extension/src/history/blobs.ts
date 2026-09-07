// The bytes of captures the app has not taken yet.
//
// `chrome.storage` serialises through JSON and cannot hold a `Blob` at all, so
// the bytes live in IndexedDB (design D2). Nothing here knows about delivery:
// it stores what it is given and deletes what it is told to.

const DB_NAME = 'boorubox-bridge'
const DB_VERSION = 1
const STORE = 'captures'

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(STORE)) {
        request.result.createObjectStore(STORE)
      }
    }
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error ?? new Error('could not open the capture store'))
  })
}

async function run<T>(
  mode: IDBTransactionMode,
  action: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await open()
  try {
    return await new Promise<T>((resolve, reject) => {
      const tx = db.transaction(STORE, mode)
      const request = action(tx.objectStore(STORE))
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error ?? new Error('capture store request failed'))
    })
  } finally {
    db.close()
  }
}

export function putBytes(id: string, blob: Blob): Promise<IDBValidKey> {
  return run('readwrite', (store) => store.put(blob, id))
}

export function getBytes(id: string): Promise<Blob | undefined> {
  return run('readonly', (store) => store.get(id) as IDBRequest<Blob | undefined>)
}

export function deleteBytes(id: string): Promise<undefined> {
  return run('readwrite', (store) => store.delete(id))
}

/**
 * Which captures still have their bytes. Read once per history render rather
 * than once per row, and never stored on an entry: site data can be cleared
 * without the entries hearing about it, and a remembered `true` would offer a
 * Retry with nothing behind it.
 */
export async function idsWithBytes(): Promise<Set<string>> {
  const keys = await run('readonly', (store) => store.getAllKeys())
  return new Set(keys.map(String))
}

// Types both runtimes need: the localhost transport contract, the site-adapter
// record shape, and the IPC contract between the app webview and the Rust core.
// Logic that decides what to do with them (tags, ratings, upload) lives in the
// app only.
//
// The Rust mirror of this file is packages/app/src-tauri/src/model.rs. Change
// one, change the other: nothing generates either from the other yet.

/** Default localhost port for the capture listener (docs/requirements.md §5). */
export const DEFAULT_PORT = 47201

/** Only requests from an extension origin may post captures (§5). */
export const EXTENSION_ORIGIN_PREFIX = 'chrome-extension://'

/**
 * The grid tile's edge in pixels: what the size slider offers and what
 * `set_grid_tile_size` clamps a stored value to (design D11). The grid reads
 * these; Rust bounds them again in `model.rs`, since a webview cannot be the
 * guard on what reaches the settings file.
 */
export const GRID_TILE_MIN = 120
export const GRID_TILE_MAX = 360
export const GRID_TILE_DEFAULT = 180

export type ImageSource = 'extension' | 'local' | 'legacy-bundle'

export type Rating = 'g' | 's' | 'q' | 'e'

/**
 * What a site adapter extracted, verbatim. The extension produces it; only the
 * app decides what the fields mean (CLAUDE.md: extraction in the extension,
 * policy in the app).
 *
 * A field value is text or a list of text, and the record is stored as it
 * arrived (design D8, D11). Normalising it on the way in would make the stored
 * record a second definition of what the adapter produced, and the extension's
 * fixture tests would stop describing what the library holds.
 */
export interface SiteAdapterRecord {
  site: string
  fields: Record<string, string | string[]>
}

/** The JSON part of a `POST /captures` multipart body. */
export interface CaptureMeta {
  /** Caller-generated UUID. Delivery is idempotent on it. */
  id: string
  imageUrl: string
  pageUrl: string
  pageTitle: string
  /** Epoch milliseconds. */
  capturedAt: number
  adapter?: SiteAdapterRecord
}

/** Body of `GET /status` when a library is open. */
export interface StatusResponse {
  version: string
  libraryPath: string
  imageCount: number
}

/** One row of the library, as the webview sees it. */
export interface ImageRecord {
  id: string
  ext: string
  mime: string
  size: number
  width: number
  height: number
  source: ImageSource
  /** Adapter site for `extension`, bundle id for `legacy-bundle`, else null. */
  sourceRef: string | null
  imageUrl: string | null
  pageUrl: string | null
  pageTitle: string | null
  /** What the capturing client's site adapter extracted, as received. */
  adapter: SiteAdapterRecord | null
  rating: Rating | null
  tags: string[]
  capturedAt: number
  createdAt: number
  updatedAt: number
  deletedAt: number | null
  /** The file under `images/` was not there at the last check. */
  missing: boolean
}

export interface TagCountFilter {
  operator: '=' | '>' | '<' | '>=' | '<=' | 'range' | 'list'
  value?: number
  values?: number[]
  min?: number
  max?: number
}

/**
 * The parsed query the webview sends to Rust, which compiles it to SQL. The
 * parser (`$lib/domain/tag-utils`) is the only definition of the query
 * language; Rust never sees the query string (design D3).
 *
 * Arrays, not Sets: this crosses the IPC boundary as JSON.
 */
export interface ParsedTagSearch {
  includeTags: string[]
  excludeTags: string[]
  orGroups: string[][]
  /** `g` | `s` | `q` | `e`. */
  ratings: string[]
  /** MIME types from `is:png` and friends. */
  fileTypes: string[]
  tagCount: TagCountFilter | null
  includeUnrated: boolean
  accounts: string[]
  excludeAccounts: string[]
}

export interface SearchRequest {
  query: ParsedTagSearch
  /** Free text matched against page title and URLs through FTS5. */
  text: string
  includeDeleted: boolean
  limit: number
  offset: number
}

export interface SearchResult {
  images: ImageRecord[]
  /** Matches before `limit`/`offset`. */
  total: number
}

export interface ImageCounts {
  total: number
  extension: number
  local: number
  legacyBundle: number
}

export interface ListenerStatus {
  running: boolean
  port: number
  /** Why the listener is not running, for settings to show (§5). */
  error: string | null
}

/** Everything the UI needs to decide between `/start` and the library. */
export interface LibraryStatus {
  opened: boolean
  libraryPath: string | null
  /** A remembered path that could not be opened; kept until another is picked. */
  missingPath: string | null
  imageCount: number
  listener: ListenerStatus
  version: string
}

/** Which palette the app paints. `system` follows the OS (design D12). */
export type Theme = 'system' | 'light' | 'dark'

/**
 * The preferences the webview reads and writes one field at a time (design D5).
 * The listener port is not here: it is already on `LibraryStatus.listener`, and
 * a second copy would be a second thing to keep in step.
 */
export interface AppSettings {
  theme: Theme
  gridTileSize: number
}

/**
 * One entry of the start screen's recent list. `name` and `available` are
 * derived from the path when the list is asked for, never stored (design D4).
 */
export interface RecentLibrary {
  path: string
  /** The folder's basename. */
  name: string
  /** `library.sqlite` is there and readable, checked at call time. */
  available: boolean
}

export type ImportStatus = 'imported' | 'skipped' | 'failed'

export interface ImportOutcome {
  path: string
  status: ImportStatus
  /** New image id when `imported`. */
  id?: string
  /** Why, when `skipped` or `failed`. */
  reason?: string
}

export interface ImportReport {
  imported: number
  skipped: number
  failed: number
  items: ImportOutcome[]
}

/** Payload of the `import:progress` event emitted while an import runs. */
export interface ImportProgress {
  done: number
  total: number
  imported: number
  skipped: number
  failed: number
}

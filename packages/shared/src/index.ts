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

/**
 * The click zoom's ceiling, as a percent of the fit: what the settings
 * slider offers and what `set_click_zoom_ceiling_percent` clamps a stored
 * value to (`click-zoom-ceiling` design D1). The webview divides this by 100
 * in exactly one place — the value handed to `clickTarget` — because a
 * float has no place in a hand-editable settings file and `AppSettings`
 * derives `Eq`, which `f64` cannot.
 */
export const CLICK_ZOOM_CEILING_MIN = 125
export const CLICK_ZOOM_CEILING_MAX = 350
export const CLICK_ZOOM_CEILING_DEFAULT = 150
/** The slider's step, a quarter of the fit: a range this narrow needs it. */
export const CLICK_ZOOM_CEILING_STEP = 25

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

/**
 * The JSON part of a `POST /captures` multipart body, and the whole body of
 * `POST /captures/pending`, where the extension announces the capture it is
 * about to fetch bytes for (design D2). One shape for both: an announcement
 * that could not be sent as the later `meta` would be a second type to mirror
 * and a placeholder that names a different page than the image it becomes.
 */
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

/**
 * Body of `DELETE /captures/pending/{id}` and payload of `capture:withdrawn`:
 * the announced capture will not arrive, so its placeholder goes.
 */
export interface CaptureWithdrawn {
  id: string
  /** Why, when the app or the extension knows; absent when nothing named one. */
  reason?: string
}

/** Body of `GET /status` when a library is open. */
export interface StatusResponse {
  version: string
  libraryPath: string
  imageCount: number
}

/**
 * One recorded post: an image has been uploaded to `site` (a `BooruSite.id`,
 * `booru-upload` design D2 — a stable slug, not a foreign key) and exists
 * there as `remoteId`. The address is computed from the site's `baseUrl`
 * (`<baseUrl>/posts/<remoteId>`), never stored, so there is one source of
 * truth for where a post lives. The record outlives the site's own
 * configuration: with no matching `BooruSite`, `posted-label` shows `site`
 * itself and omits the link.
 */
export interface PostRef {
  site: string
  remoteId: string
  /** Epoch milliseconds. */
  postedAt: number
}

/** One row of the library, as the webview sees it. */
export interface ImageRecord {
  id: string
  ext: string
  /**
   * Path to the file, relative to the library root, `/`-separated on every
   * platform: `images/<a1>/<b2>/<id>.<ext>`. What `imageUrl` joins onto the
   * library path — the webview never composes the layout itself.
   */
  file: string
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
  /**
   * The X account `pageUrl` names, derived from it by the same rule the
   * search's `account:` filter matches on (Rust's `query::x_account`),
   * computed on every load rather than stored. `null` for a page that names
   * no X account.
   */
  account: string | null
  /** What the capturing client's site adapter extracted, as received. */
  adapter: SiteAdapterRecord | null
  rating: Rating | null
  tags: string[]
  capturedAt: number
  /**
   * The file's own modification time, epoch milliseconds; absent for an
   * image that never came from a file.
   */
  fileModifiedAt: number | null
  createdAt: number
  updatedAt: number
  deletedAt: number | null
  /** The file under `images/` was not there at the last check. */
  missing: boolean
  /**
   * Where this image has been posted, one entry per site (`booru-upload`
   * design D5). Empty for an image never posted anywhere.
   */
  posts: PostRef[]
  /**
   * The collections this image is in, by id, sorted (`collections` design
   * D4). Never a tag: not sent to a booru, not matched by a tag term.
   */
  collections: string[]
}

/**
 * A named, unordered set of images the user keeps for themselves
 * (`collections` design D1–D3): favourites, a project, a batch to upload.
 * Referenced everywhere else by `id`, which never changes; `name` is the only
 * field a rename touches.
 */
export interface Collection {
  id: string
  name: string
  /**
   * `name`, lower-cased with runs of whitespace as `_` (design D2), computed
   * once in Rust — the webview never recomputes it, so the query language and
   * the sidebar always agree on what a collection is called.
   */
  slug: string
  /** Epoch milliseconds. */
  createdAt: number
  updatedAt: number
}

/**
 * One collection's row in the sidebar and the inspector's menus: the
 * collection plus how many of the matched result are in it (design D7).
 */
export interface CollectionCount {
  id: string
  name: string
  slug: string
  count: number
}

/**
 * What `updateFacts` sends (`editable-info` design D1). `null` and an empty
 * string both mean "clear this field" — Rust trims and treats either the same
 * way, so the form has no separate "clear" affordance to get out of sync with
 * what typing nothing does. Every field is required (never `undefined`): Rust
 * has no `#[serde(default)]` on these, since the whole form is always sent
 * together.
 */
export interface FactsEdit {
  pageTitle: string | null
  pageUrl: string | null
  imageUrl: string | null
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
  /**
   * Slugs (design D6): `collection:my_favorites` compiles against
   * `collections.slug`, never the id — Rust never sees one from the webview.
   */
  collections: string[]
  excludeCollections: string[]
}

/**
 * What the sorts compare. Rust maps each to a fixed column expression.
 * `trashed` orders by the time of trashing and is only offered in the trash
 * view: on a library row that time is NULL.
 */
export type SortField = 'captured' | 'updated' | 'size' | 'dimensions' | 'trashed'

export type SortDirection = 'asc' | 'desc'

export interface Sort {
  field: SortField
  direction: SortDirection
}

/**
 * How the result set is divided. A grouping also restricts it: `x-account`
 * admits only pages naming an account, `duplicates` only images sharing their
 * dimensions and byte size with another (design D7).
 */
export type GroupBy = 'none' | 'x-account' | 'duplicates'

/**
 * One group of the whole result set, in the order the result is returned in.
 * `key` is raw — the account handle, or `WIDTHxHEIGHT-SIZE` — because a display
 * label is UI and the storage layer does not write one (design D7).
 */
export interface GroupSlice {
  key: string
  count: number
}

export interface SearchRequest {
  query: ParsedTagSearch
  /** Free text matched against page title and URLs through FTS5. */
  text: string
  /**
   * Which set of images the request draws from: the library (deleted images
   * excluded) or the trash (deleted images only, `trash` design D1, D2). Never
   * both — nothing in the app shows the library and the trash at once.
   */
  view: 'library' | 'trash'
  sort: Sort
  group: GroupBy
  limit: number
  offset: number
}

export interface SearchResult {
  images: ImageRecord[]
  /** Matches before `limit`/`offset`. */
  total: number
  /** Every group of the result; empty when ungrouped. */
  groups: GroupSlice[]
}

export interface TagCount {
  name: string
  count: number
}

/**
 * What `exportZip` answers with (`selection-and-bulk` design D11): how many of
 * the selection made it into the archive, and which ids did not because their
 * file was gone from `images/` by the time it was copied.
 */
export interface ExportReport {
  path: string
  written: number
  missing: string[]
}

/**
 * Payload of the `export:progress` event, emitted once per id including the
 * ones left out — so the last tick's `done` always equals `total`, mirroring
 * `ImportProgress` (design D13).
 *
 * Also the payload of every other `{ done, total }` event: `rules:progress`
 * (`auto-tag-rules` D12), `library:rebuild` and `library:sidecars`
 * (`library-sidecars` D13), each of which has its own name on the Rust side.
 * A field added here for the export's sake would therefore be a field three
 * other subscribers type and never receive — give that event its own
 * interface instead.
 */
export interface ExportProgress {
  done: number
  total: number
}

/**
 * What `deleteForever` and `emptyTrash` answer with (`trash` design D4, D7):
 * how many images were permanently removed, and the full path of every file
 * that could not be unlinked — named on screen rather than swept, since the
 * record is gone either way and there is no unlink left to retry.
 */
export interface DeleteReport {
  deleted: number
  filesLeft: string[]
}

/**
 * One sidecar `rebuildLibrary` could not read (`library-sidecars` design
 * D11): a malformed document is counted and named rather than taking the
 * whole rebuild down with it, the same rule `ImageRecord`'s own reader
 * applies to a bad `adapter` field.
 */
export interface RebuildFailure {
  file: string
  reason: string
}

/**
 * What `rebuildLibrary` answers with (design D10–D13): how many images came
 * back, every sidecar that could not be read, the name the previous
 * database was kept under (never deleted), and how many rules and sites
 * came back from `library.json`.
 */
export interface RebuildReport {
  images: number
  failed: number
  failures: RebuildFailure[]
  /** Where the old database was moved; empty when there was none to keep. */
  keptAs: string
  rules: number
  sites: number
  collections: number
}

export interface RatingCounts {
  g: number
  s: number
  q: number
  e: number
  unrated: number
}

/**
 * What the sidebar draws. The two halves answer different questions and are
 * counted differently on purpose (design D8): `tags` over the fully filtered
 * result, `ratings` with the rating clause dropped, so a pill says how many the
 * search would return if that rating were asked for instead.
 */
export interface TagCounts {
  tags: TagCount[]
  ratings: RatingCounts
  /**
   * Every collection in the library, counted over the matched set with the
   * rating clause included (design D7) — the sidebar's Collections section
   * and the inspector's badges read this rather than keeping a count of
   * their own.
   */
  collections: CollectionCount[]
  /**
   * Every account of the request's view — including one with no matches —
   * when `group` is `x-account`; empty otherwise, since the rail this backs
   * is shown only then and the pass over every page address is not free
   * enough to run when it is not shown.
   *
   * Each count drops the query's own `account:`/`-account:` clause and
   * honours everything else — the rating pills' rule (design D8 of
   * `tags-and-ratings`, "how many if I switched"), applied to accounts:
   * while `account:alice` narrows the search, `bob`'s count still answers
   * what he would give if the search switched to him. Ordered by count
   * descending then handle ascending, the order `GroupSlice`s for an account
   * grouping already use.
   */
  accounts: GroupSlice[]
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
  /**
   * The path a launch-time open is still running against (`launch-screen`
   * design D1) — set while the remembered library's open runs off the main
   * thread, `null` once it settles either way. Lets the opening screen name
   * the folder before `opened` can be true.
   */
  opening: string | null
  libraryPath: string | null
  /** A remembered path that could not be opened; kept until another is picked. */
  missingPath: string | null
  /**
   * A remembered path that could not be opened because its database is
   * damaged (`library-sidecars` design D9) — set only for that one reason.
   * Every other reason a library failed to open, including a folder that is
   * simply gone, keeps `missingPath` instead: the start screen has to tell
   * "damaged" from "gone" apart, and re-deriving that on every status poll
   * would mean a `quick_check` per poll, so it is recorded by the attempt
   * that met it rather than guessed from the folder.
   */
  damagedPath: string | null
  /**
   * A remembered path that could not be opened because a newer build of
   * BooruBox wrote it (`library-recovery`'s "A library from a newer build").
   * Its own slot rather than a silence: a status naming none of the three
   * reads on the start screen as "choose a library folder", which loses the
   * folder the user already has and says nothing about why it will not open.
   * A rebuild is never offered for one — rebuilding it would downgrade it.
   */
  newerPath: string | null
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
  /**
   * How far a click in the viewer may zoom, as a percent of the fit
   * (`click-zoom-ceiling` design D1): the click's target is the cover or
   * this ceiling times the fit, whichever is smaller. A percent, not a
   * float: Rust's `AppSettings` derives `Eq`, which `f64` cannot, and a float
   * in `settings.json` invites `2` against `2.0` against `1.9999`.
   */
  clickZoomCeilingPercent: number
  /**
   * Whether the sidebar's notes panel is folded away (`notes` design D13).
   * A preference held for months rather than a view toggle, which is why it
   * is here and not session state — that decision reverses app-shell's, and
   * D13 records why the old reading was right until the panel existed.
   */
  notesCollapsed: boolean
  /**
   * Whether the sidebar's collections section is folded away
   * (`browse-feedback` design D4), the same kind of preference as
   * `notesCollapsed` and copied from its line.
   */
  collectionsCollapsed: boolean
  /**
   * Whether the app reopens `LibraryStatus.libraryPath`'s remembered folder
   * automatically at launch, or waits on the start screen for the user to
   * pick one (`launch-screen` design D3). Takes effect at the next launch,
   * not immediately.
   */
  openLastOnLaunch: boolean
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
  /**
   * Stopped by Cancel rather than finishing on its own
   * (`import-pause-cancel` design D3). The counts above are real either way
   * — cancelling never rolls back what already landed.
   */
  cancelled: boolean
  items: ImportOutcome[]
}

/**
 * One bundle part as `bundle_plan` found it, before any row is read: either
 * its row count, or why it would not open. Exactly one of `rows` and `error`
 * is set.
 */
export interface BundlePartPlan {
  path: string
  rows: number | null
  error: string | null
}

/**
 * What `bundle_plan` answers with: every picked part, in the order the run
 * will read them, and `total` — the same number the run's first
 * `import:progress` carries, computed once so the confirm screen and the run
 * never disagree about how much work this is.
 */
export interface BundlePlan {
  parts: BundlePartPlan[]
  total: number
}

/** Payload of the `import:progress` event emitted while an import runs. */
export interface ImportProgress {
  done: number
  total: number
  imported: number
  skipped: number
  failed: number
}

/**
 * A named pattern and the tags it adds (`auto-tag-rules` design D1, D2). Rules
 * live in the library, not `settings.json`: library policy travels with the
 * folder. `tags` is a rule's payload, not tag rows — it is never searched,
 * counted or joined, only written later (design D2).
 */
export interface Rule {
  id: string
  name: string
  pattern: string
  isRegex: boolean
  tags: string[]
  enabled: boolean
  /** Epoch milliseconds. */
  createdAt: number
  updatedAt: number
}

/** What `rules_upsert` takes: `id` absent creates, present edits (design D6). */
export interface RuleInput {
  id?: string
  name: string
  pattern: string
  isRegex: boolean
  tags: string[]
  enabled: boolean
}

/**
 * One row of the rules list: the rule plus its pattern's validity, compiled at
 * read time and never stored (design D6) — a regular expression the engine
 * cannot use is reported here rather than dropped.
 */
export interface RuleListEntry {
  rule: Rule
  patternError: string | null
}

/** What `rules_import` answers with (design D10). */
export interface RulesImportReport {
  imported: number
  skipped: number
}

/**
 * One rule's outcome in a `rules_run`: how many images it matched, in the
 * `rules` half of {@link RulesRunReport}, or its `patternError` in the
 * `invalid` half — the same shape serves both (design D9).
 */
export interface RuleRunCount {
  id: string
  name: string
  matched: number
  patternError?: string
}

/**
 * What `rules_run` answers with: how many images were examined and changed,
 * and each rule's own count, invalid ones named separately with their reason
 * rather than dropped (design D9).
 */
export interface RulesRunReport {
  examined: number
  changed: number
  rules: RuleRunCount[]
  invalid: RuleRunCount[]
}

/**
 * The library's one free-text scratchpad (`notes` design D3). A library that
 * has never been written to reads as an empty note, never an error.
 */
export interface Note {
  content: string
  updatedAt: number
}

/**
 * A booru this library posts to (`booru-upload` design D1, D2, `booru-sites`).
 * Lives in `library.sqlite`; the API key never does — it goes to the OS
 * credential store, keyed on `<host>/<username>` (design D7), and is never
 * part of this type.
 *
 * `id` is a slug derived once from `baseUrl`'s host at creation and never
 * changes afterwards (design D2), even if `baseUrl` is edited later: it is
 * what `PostRef.site` and the `posts` table hold, and it must outlive the row
 * it was derived from.
 */
export interface BooruSite {
  id: string
  name: string
  baseUrl: string
  username: string
  createdAt: number
  updatedAt: number
}

/**
 * What `booru_site_test` answers with (design D8): the connection test is
 * judged by status code alone, never by the response body, so a self-hosted
 * fork's own profile shape never fails a test that a real Danbooru would pass.
 */
export type BooruConnectionTest
  = | { status: 'connected' }
    | { status: 'credentialRejected' }
  /** The transport's own reason, or the status the site answered with. */
    | { status: 'unreachable', reason: string }

/**
 * The upload form's values, prefilled per `booru-upload` design D10 and
 * editable before sending. `rating` is never unset here: the form (webview
 * side) refuses to send until one is chosen, so by the time this crosses IPC
 * it is always one of the four letters — an unrated image simply starts the
 * form with nothing preselected.
 */
export interface BooruUploadForm {
  /** Space-joined at the request builder, not here — see `create_post`. */
  tags: string[]
  rating: Rating
  /** The page the image was captured from, or the image's own address. */
  source: string
  /** Empty when no rule matched the page address (design D11). */
  artist: string
  commentaryTitle: string
  commentaryBody: string
}

/**
 * The step an upload sequence failed at (design D6): `authenticate` is the
 * booru itself rejecting the credential (a 401/403 on any of the four calls),
 * never a local credential-store failure — that fails the command outright
 * (`AppError`) before any request is sent (`booru-sites` design D7).
 */
export type UploadStep = 'authenticate' | 'createUpload' | 'awaitProcessing' | 'createPost' | 'commentary'

/**
 * Which step failed, the booru's own message where it gave one (design D6 —
 * never a paraphrase), and the remote reference a failure can point at: the
 * upload id when processing times out, the post id when only the commentary
 * fails.
 */
export interface UploadStepError {
  step: UploadStep
  message: string
  remoteRef?: string
}

/**
 * Whether the optional artist commentary call ran, and how it went. Its own
 * failure is not the upload's failure (design D6): the post already exists,
 * so `BooruUploadOutcome` still reports `Posted` with this as a warning.
 */
export type CommentaryOutcome
  = | { status: 'skipped' }
    | { status: 'applied' }
    | { status: 'failed', message: string }

/**
 * What `booru_upload` answers with. Only `Posted` ever comes with a `PostRef`
 * — nothing is recorded against the image for any `Failed` outcome, however
 * far the sequence got (design D5).
 */
export type BooruUploadOutcome
  = | { outcome: 'posted', post: PostRef, commentary: CommentaryOutcome }
    | { outcome: 'failed', error: UploadStepError }

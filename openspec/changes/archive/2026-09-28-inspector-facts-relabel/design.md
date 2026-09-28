## Context

`Inspector.svelte` renders the facts as a `<dl>` (rows from "Title" to "ID", around lines
1311–1480); `format.ts` holds the pure formatters (`formatBytes`, `formatTimestamp`,
`ratingLabel`) with `format.test.ts`, because the inspector has no component test harness.
`ImageRecord` carries `file` (`images/<a>/<b>/<id>.<ext>`), `mime`, `width`, `height`,
`size`, `capturedAt`, `createdAt`, `fileModifiedAt` (nullable). `TagNoteIndicator.svelte` is
the app's one hover tooltip: bits-ui `Tooltip.Root delayDuration={150}` with a `<span>`
trigger and `portalProps={{ to: portalTo }}` so it renders inside the viewer placement too.

## Decisions

**D1. Labels: Origin, Source, Source file.** "Origin" is the one word that fits a capture, a
local import and a bundle alike ("Import from" reads wrong for a live capture); "Source" is
Danbooru's word for the page and the filter's; "Source file" pairs with it as the file that
page embeds. The `aria-label`s of the two inputs follow: "Source address", "Source file
address".

**D2. One Size row, formatted by a pure function.** `formatSizeLine({ size, file, width,
height })` in `format.ts` returns `${formatBytes(size)} .${ext} (${width}×${height})`, where
`ext` is the text after the last `.` in `file`'s last path segment, lower-cased; with no
extension the middle term is omitted (`1.3 MB (1200×2200)`). The extension comes from the
stored filename, not `mime`, so it reads `.jpg` rather than `image/jpeg`; `×` stays, the
panel's existing glyph. Tests: a jpg, a file with no extension, an upper-case extension.

**D3. One Date row, relative, with the absolute times on hover.** `formatRelative(ms, now)`
in `format.ts`: `Intl.RelativeTimeFormat('en', { numeric: 'auto' })` over the largest unit
whose magnitude is at least one (seconds → "just now" under a minute, then minutes, hours,
days, months at 30 days, years at 365), so it reads `yesterday`, `3 days ago`, `2 years ago`.
`now` is a parameter for the tests and defaults to `Date.now()`; a non-finite `ms` reads `—`
like `formatTimestamp`. The row reads `formatRelative(image.capturedAt)`, computed when the
image is derived, not ticking. The hover is a `Tooltip.Root delayDuration={150}` copied in
shape from `TagNoteIndicator` (span trigger, `tabindex={-1}`, the same `portalProps`), whose
content is three lines: `Captured <absolute>`, `Imported <absolute>`, `File modified
<absolute or —>` through `formatTimestamp`. A tooltip, not a `title` attribute: the native
one is slow and unstyled and the app already has the styled one.

**D4. The spec names the rows.** The "at least" sentence keeps every fact (dimensions, size,
type, capture time are all still shown, two of them inside one row) and the order sentence
reads "the facts (origin, source, source file, size, date, id)". Scenarios that name "the
page address" keep the phrase: it is what the row holds, whatever its label.

## Risks

- The tooltip's portal target inside the viewer: `TagNoteIndicator` already solves it with
  `portalTo`; the Date row uses the same prop the inspector already passes to tag notes.

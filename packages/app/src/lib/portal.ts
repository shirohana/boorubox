/**
 * Where an overlay portals to (design D3 of `browse-fixes`): the nearest
 * native modal dialog, or `<body>` when there is none. `showModal()` puts a
 * dialog in the browser's top layer, which sits above every z-index, so a
 * list portalled to `<body>` while one is open lands beneath it — visible
 * through the backdrop and unreachable. Inside one, the overlay has to be a
 * child of the dialog itself.
 */
export function portalTarget(el: Element | null | undefined): Element | undefined {
  return el?.closest('dialog') ?? undefined
}

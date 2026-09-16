> One Sonnet unit, all packages — the shared type, the Rust field and the webview are one wire
> and cannot be split. Gate: `mise run lint`, `mise run typecheck`, `mise run test` (both vitest
> and cargo), `mise run clippy`. Design D1–D4 decide every shape below; do not re-decide them.

## 1. The contract and the store (`packages/shared`, `packages/app/src-tauri`)

- [x] 1.1 `packages/shared/src/index.ts`: `clickZoomCeilingPercent: number` on `AppSettings`
      (doc comment: what it is, D1's reason for a percent) and `CLICK_ZOOM_CEILING_MIN = 150`,
      `_MAX = 500`, `_DEFAULT = 200` beside `GRID_TILE_*`. Verify: `pnpm -r typecheck` now fails
      in exactly the three fixtures D's Risks name — that is the checkpoint, fixed in 2.1.
- [x] 1.2 `model.rs`: the three constants beside `GRID_TILE_*` (with the tile size's comment
      about which three decide), `click_zoom_ceiling_percent: u32` on `AppSettings` with a doc
      comment, and the wire test `app_settings_crosses_the_wire_in_camel_case` extended with
      `"clickZoomCeilingPercent": 200`. Verify: `cargo test app_settings_crosses` passes.
- [x] 1.3 `settings.rs`: the field on `Settings` (default `CLICK_ZOOM_CEILING_DEFAULT`), the
      `clickZoomCeilingPercent` key, `From<&Settings>`, load (out of range or missing → default,
      copied from `grid_tile_size`'s lines and reason) and save; a test
      `a_click_zoom_ceiling_outside_the_range_reads_as_the_default` copied from the tile size's,
      and the round-trip test extended. Verify: `cargo test settings::` passes.
- [x] 1.4 `commands.rs` + `lib.rs`: `set_click_zoom_ceiling_percent(percent: u32, …) ->
      Result<AppSettings>` clamping to the range through `write_settings`, registered in the
      handler list after `set_grid_tile_size`; a test copied from
      `a_tile_size_past_the_range_is_clamped_rather_than_refused`. Verify: `cargo test
      click_zoom` passes, `mise run clippy` clean.

## 2. The webview (`packages/app`)

- [x] 2.1 `api/commands.ts`: `setClickZoomCeilingPercent(percent)` invoking the command;
      `api/settings.svelte.ts`: `setClickZoomCeilingPercent` storing the answer; the fixtures in
      `commands.test.ts` and `settings.svelte.test.ts` gain the field, and each file gains one
      test copied from the tile size's (`set_grid_tile_size` shape / the clamped-answer shape).
      Verify: `pnpm -r typecheck` green again, `pnpm --filter @boorubox/app test` green.
- [x] 2.2 `viewer-zoom.ts`: `clickTarget(natural, viewport, ceiling)` per D2, the doc comment
      restated; `viewer-zoom.test.ts`: the two existing `clickTarget` tests pass a ceiling of 8
      (unchanged answers) and four new ones pin the spec's scenarios — portrait 1000×3000 in
      1500×900 at 2 → 0.6; the same at 5 → 1.5 (the cover); 400×300 at 2 → 2; the matched-aspect
      3000×1800 at 1.5 → 0.75 (the fallback capped). Verify: `pnpm --filter @boorubox/app test
      viewer-zoom` passes.
- [x] 2.3 `Lightbox.svelte`: a `clickZoomCeiling: number` prop (doc: a multiple of the fit,
      D3), passed as the third argument in `toggleZoom`. `LibraryScreen.svelte`: a `$derived`
      from `settings.current?.clickZoomCeilingPercent ?? CLICK_ZOOM_CEILING_DEFAULT`, divided by
      100, passed to `<Lightbox>`. Verify: typecheck and lint green.
- [x] 2.4 `routes/settings/+page.svelte`: under the thumbnail-size block, a labelled `Slider`
      (`id="click-zoom"`, label "Click zoom — up to {percent / 100}× the fit", min/max/step from
      the shared constants, step 50) with a `commitClickZoom` copied from `commitTileSize`.
      Verify: lint green; the whole gate green.
      Hand check: settings shows "up to 2× the fit"; drag to 5×, open a portrait image, click —
      it covers the width; back to 2× — the click stops at twice the fit with dark space beside;
      restart the app — the slider still reads what was set.
      Seen by the lead on a scratch copy of test-1 (2026-09-17, driven through accessibility, not
      the owner's hands): the label read "up to 2× the fit" and, after End on the slider, "up to
      5× the fit", with `settings.json` holding 200 then 500; a 2200×3240 image in the 1200×800
      window, gallery mode, clicked at 2× stopped with dark space at both sides and at 5× covered
      the width. The restart half was not run; the load path is covered by the settings tests.

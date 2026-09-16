> Lead, webview only. Gate: `pnpm -r typecheck`, `pnpm lint`, `pnpm --filter @boorubox/app test`.

## 1. The viewer

- [x] 1.1 `Lightbox.svelte`: dialog `h-screen w-screen`, viewport `inset-0`, the margin's comments
      gone (design D1). Verify: the gate.
      Hand check: open a portrait image — it touches the top and bottom edges at the fit; click —
      it covers the width edge to edge; Escape closes; at the fit a click beside the image closes.

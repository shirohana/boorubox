## Context

`StampBar.svelte` wraps its `TagInput` in a `relative min-w-0 flex-1` div and, while `text !==
''`, mounts a ghost `icon-xs` `Button` with `class="absolute top-1/2 right-0.5
-translate-y-1/2"` whose `onclick` empties the field and calls `focusEnd()` on it
(`stamp-bar-clear` D2). The kit's `Button` base class (`ui/button/button.svelte`) carries
`active:not-aria-[haspopup]:translate-y-px`: every button nudges itself one pixel down while
pressed. Tailwind's translate utilities all write the one `translate` property, so on a
pressed clear control `translate-y-px` replaces `-translate-y-1/2` and the control drops from
half its height above the centre line to one pixel below it — about 13px for a 24px button.
The pointer that pressed it at its centre is then at its top edge or past it; the release
outside the element fires no `click`; `clear()` never runs. In WebView2 the press had already
moved focus from the field to the button, so the field ends blurred with its text intact. It is
the only button in the app positioned this way (`grep translate-y-1/2` outside `ui/`).

## Decisions

**D1. The centring lives on a wrapper, not on the button.** The button is placed by a
`<div class="absolute inset-y-0 right-0.5 flex items-center">` and loses its own
positioning classes. The wrapper carries a comment naming the reason: the kit's button moves
itself on press through the same translate, so a centring translate on the button is lost
exactly while it is pressed. Nothing else in the bar changes; `clear()` and `focusEnd()` stay
as they are, since once the click fires they already do what the requirement asks.

**D2. No jsdom assertion on the classes.** The defect is a CSS cascade under `:active`, which
jsdom does not compute; a test that asserts the button's class string would pin markup, not
behaviour. The existing test (`offers the clear control only while the field holds text, and
it empties the field`) still covers the click's effect; the press is proven on screen by the
lead with the mouse helper on macOS and by the owner on Windows.

## Risks / Trade-offs

- [The wrapper intercepts clicks beside the button] → it is `flex items-center` around a
  24px button inside the field's `pr-7` reservation; the strip beside the button is 2px of
  padding and lands in the field by `pointer-events` on the input underneath only if the
  wrapper gets `pointer-events-none` with the button `pointer-events-auto`. Add both classes
  so a click in the reservation's margin still focuses the field.

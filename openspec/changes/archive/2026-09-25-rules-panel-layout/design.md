## Context

`RulesTable.svelte` (`auto-tag-rules` design D11, "the legacy's table") renders `Table.*` in an
`overflow-x-auto` box; `RulesSection.svelte` mounts it under the form. The settings page is a
`max-w-2xl` column. `StampsTable.svelte` is the sibling with the same shape and fewer columns.

## Decisions

**D1. A list of entries, not a table with fewer columns.** The component is renamed
`RuleList.svelte` (a table that is not a table would lie in its name), rendering `<ul
class="flex flex-col gap-2">` with one `<li class="rounded-lg border border-border p-3">` per
rule. Header row: `<div class="flex flex-wrap items-center gap-2">` — the name (`font-medium
text-sm`), the New badge, the regex badge, then `ml-auto` the `Switch` and the edit and delete
buttons as `size="icon-xs" variant="ghost"`. The invalid reason stays a `text-xs
text-destructive` line under the header, exactly the text it is today. Body: a `<dl
class="mt-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">` with two rows,
`Pattern` and `Tags`, `dt` in `text-muted-foreground`, `dd` `min-w-0`: the pattern as
`font-mono break-all` or the italic "(matches all)", the tags as the same `Badge
variant="secondary"` chips wrapping. The five-column table had nothing the header plus two
labelled rows do not; the labels replace the column headings.

**D2. Behaviour is moved, not rewritten.** `setEnabled`, `run`, the delete confirmation and the
three props stay as they are; only the markup changes. `RulesSection.svelte` changes its import
and mount. The D11 note "the legacy's table" is amended in the archive, not here.

## Risks / Trade-offs

- [A long rule name and the controls on one line] → `flex-wrap`; the controls drop to a second
  line rather than pushing the box wider.

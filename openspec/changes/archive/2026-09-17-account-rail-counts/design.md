## Context

See proposal.md — Why. The rail (`browse-feedback` design D9) draws `results.groups`, which
`query.rs` computes over the matched set *as filtered*, so an included account is the only
group left. The rating pills already answer the sideways question — `tag_counts` compiles a
second `Plan` with `RatingClause::Dropped` (design D8 of `tags-and-ratings`) — and the
grouped plan's `slices` CTE already holds per-account sizes over the matched set.

## Goals / Non-Goals

**Goals:** one more count on the existing counts round trip, on the pills' rule; the rail a
pure reader of it; no change to the grid's groups.

**Non-Goals:** a store of accounts; counting when not grouped by account.

## Decisions

### D1. The list is a fourth field of `TagCounts`, computed with the account clause dropped

`compile` takes an `AccountClause` beside `RatingClause`; `Plan::for_account_counts(req)`
drops it (rating included), and `account_counts` reads `slices` left-joined onto every
distinct `x_account(page_url)` of the view, so accounts with no match come back at zero.
Filled only while `req.group` is `x-account`: the rail is only shown then, and the distinct
pass over page addresses is a full scan of the view. Reuses `GroupSlice` (`key` = handle).
*Alternative rejected:* a separate command from the rail — two round trips describing two
queries, the hazard `tag_counts`'s comment already names.

### D2. The rail sorts the search's own accounts first and keeps its rows through a search

`AccountRail` takes `accounts: GroupSlice[] | null` from `results.counts` and orders
included/excluded first (stable, so Rust's largest-first order holds within each half — the
tag sidebar's rule). `counts` is `null` for the length of a search; the rail keeps the last
list's handles as its rows and blanks the numbers meanwhile (design D8's honest blank), the
fix `CollectionsSection` got for the same scroll reset in `browse-feedback`.

## Risks / Trade-offs

- [A full pass over page addresses per search while grouped] → grouping by account already
  makes that pass; the distinct list is a second one, milliseconds on 25k rows.
- [The rail's counts and the grid's headings disagree while an account is included] → by
  design, as the pills and the grid do; the rail answers "if I switched".

import type { RuleListEntry } from '@boorubox/shared'

/**
 * The entries whose name, pattern or any tag contains the query, ignoring case.
 * A blank query answers `entries` itself, so an unfiltered list keeps its identity.
 */
export function filterRules(entries: RuleListEntry[], query: string): RuleListEntry[] {
  const needle = query.trim().toLowerCase()
  if (needle === '') return entries
  const has = (text: string) => text.toLowerCase().includes(needle)
  return entries.filter(({ rule }) => has(rule.name) || has(rule.pattern) || rule.tags.some(has))
}

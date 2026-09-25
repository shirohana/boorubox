<script lang="ts">
  // Slot Settings · Rules, the listing half (`auto-tag-rules` design D11): a
  // stacked list of entries, not a table — the settings column is 672px and a
  // table's five columns (name, pattern, tags, on, actions) never fit it
  // (`rules-panel-layout` design D1). Ordered by name in Rust (design D4) so
  // nothing here sorts.
  import type { RuleListEntry } from '@boorubox/shared'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { errorText, rulesDelete, rulesUpsert } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { Switch } from '$lib/components/ui/switch'

  interface Props {
    entries: RuleListEntry[]
    /**
     * The ids an import just added, marked until the section is left — the
     * legacy's NEW badge, and the only way to see what an import of thirty
     * rules actually put in the list (design D11).
     */
    newIds: Set<string>
    onedit: (entry: RuleListEntry) => void
    /** A write was attempted: the caller re-reads the list, landed or not. */
    onchanged: () => void | Promise<void>
    onerror: (message: string) => void
  }

  let { entries, newIds, onedit, onchanged, onerror }: Props = $props()

  let confirming = $state<RuleListEntry | null>(null)

  function deleteConfirmed() {
    const doomed = confirming
    confirming = null
    if (doomed) void run(() => rulesDelete(doomed.rule.id))
  }

  async function run(action: () => Promise<unknown>) {
    let failure: string | null = null
    try {
      await action()
    } catch (cause) {
      failure = errorText(cause)
    }
    // The list comes back whether the write landed or not: the switch below
    // flips itself on click, so a refused toggle would go on showing a state
    // the library does not hold. The reason is reported after the re-read,
    // because the re-read is what clears the section's last error.
    await onchanged()
    if (failure !== null) onerror(failure)
  }

  /**
   * Enabling and disabling goes through the same upsert as an edit: `enabled`
   * is a column of the rule, and a command of its own would be a second write
   * path to the same row. The upsert validates a pattern only when the edit
   * rewrites it, so a rule imported with a regular expression this app cannot
   * compile (design D6) can still be switched off from here.
   */
  function setEnabled(entry: RuleListEntry, enabled: boolean) {
    const { id, name, pattern, isRegex, tags } = entry.rule
    void run(() => rulesUpsert({ id, name, pattern, isRegex, tags, enabled }))
  }
</script>

{#if entries.length === 0}
  <p class="text-sm text-muted-foreground">
    No rules yet. A rule adds its tags to every image whose title or capture details match its
    pattern, as the image enters the library.
  </p>
{:else}
  <ul class="flex flex-col gap-2">
    {#each entries as entry (entry.rule.id)}
      <li class="rounded-lg border border-border p-3">
        <div class="flex flex-wrap items-center gap-2">
          <span class="min-w-0 text-sm font-medium wrap-break-word">{entry.rule.name}</span>
          {#if newIds.has(entry.rule.id)}
            <Badge variant="secondary">New</Badge>
          {/if}
          {#if entry.rule.isRegex}
            <Badge variant="outline">regex</Badge>
          {/if}
          <div class="ml-auto flex items-center gap-1">
            <Switch
              checked={entry.rule.enabled}
              aria-label="Enable {entry.rule.name}"
              onCheckedChange={(enabled) => setEnabled(entry, enabled)}
            />
            <Button
              size="icon-xs"
              variant="ghost"
              aria-label="Edit {entry.rule.name}"
              onclick={() => onedit(entry)}
            >
              <PencilIcon />
            </Button>
            <Button
              size="icon-xs"
              variant="ghost"
              aria-label="Delete {entry.rule.name}"
              onclick={() => (confirming = entry)}
            >
              <Trash2Icon />
            </Button>
          </div>
        </div>

        <!--
          The engine's own message, not a paraphrase: a pattern written
          against another regular-expression engine is imported rather
          than dropped (design D6), and this is where its reason is read.
        -->
        {#if entry.patternError}
          <p class="mt-0.5 text-xs text-destructive">Invalid: {entry.patternError}</p>
        {/if}

        <dl class="mt-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">
          <dt class="text-muted-foreground">Pattern</dt>
          <dd class="min-w-0">
            <!--
              An empty pattern matches every image, so the list says so
              rather than showing an empty value (spec: "The pattern that
              matches everything").
            -->
            {#if entry.rule.pattern === ''}
              <span class="text-muted-foreground italic">(matches all)</span>
            {:else}
              <code class="font-mono break-all">{entry.rule.pattern}</code>
            {/if}
          </dd>

          <dt class="text-muted-foreground">Tags</dt>
          <dd class="min-w-0">
            <ul class="flex flex-wrap gap-1">
              {#each entry.rule.tags as tag (tag)}
                <li><Badge variant="secondary">{tag}</Badge></li>
              {/each}
            </ul>
          </dd>
        </dl>
      </li>
    {/each}
  </ul>
{/if}

<!--
  A rule is not recoverable once deleted and the pattern behind it may have
  taken a while to write, so this asks. It says what deleting does not do,
  because that is the question: the images it tagged keep their tags (spec
  "Deleting").
-->
<ConfirmDialog
  title="Delete “{confirming?.rule.name}”?"
  description="The rule is gone and stops applying to arriving images. Every image it has
    already tagged keeps its tags."
  confirmLabel="Delete rule"
  open={confirming !== null}
  onclose={() => (confirming = null)}
  onconfirm={deleteConfirmed}
/>

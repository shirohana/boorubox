<script lang="ts">
  // Slot Settings · Rules, the listing half (`auto-tag-rules` design D11): the
  // legacy's table, ordered by name in Rust (design D4) so nothing here sorts.
  import type { RuleListEntry } from '@boorubox/shared'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { errorText, rulesDelete, rulesUpsert } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { Switch } from '$lib/components/ui/switch'
  import * as Table from '$lib/components/ui/table'

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
  <div class="overflow-x-auto rounded-lg border border-border">
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head>Pattern</Table.Head>
          <Table.Head>Tags</Table.Head>
          <Table.Head class="w-16">On</Table.Head>
          <Table.Head class="w-20"></Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each entries as entry (entry.rule.id)}
          <Table.Row>
            <Table.Cell class="align-top">
              <div class="flex flex-wrap items-center gap-1">
                <span class="font-medium">{entry.rule.name}</span>
                {#if newIds.has(entry.rule.id)}
                  <Badge variant="secondary">New</Badge>
                {/if}
              </div>
              <!--
                The engine's own message, not a paraphrase: a pattern written
                against another regular-expression engine is imported rather
                than dropped (design D6), and this is where its reason is read.
              -->
              {#if entry.patternError}
                <p class="mt-0.5 text-xs text-destructive">Invalid: {entry.patternError}</p>
              {/if}
            </Table.Cell>

            <Table.Cell class="align-top">
              <div class="flex flex-wrap items-baseline gap-1">
                <!--
                  An empty pattern matches every image, so the list says so
                  rather than showing an empty cell (spec: "The pattern that
                  matches everything").
                -->
                {#if entry.rule.pattern === ''}
                  <span class="text-muted-foreground italic">(matches all)</span>
                {:else}
                  <code class="font-mono text-xs break-all">{entry.rule.pattern}</code>
                {/if}
                {#if entry.rule.isRegex}
                  <Badge variant="outline">regex</Badge>
                {/if}
              </div>
            </Table.Cell>

            <Table.Cell class="align-top">
              <ul class="flex flex-wrap gap-1">
                {#each entry.rule.tags as tag (tag)}
                  <li><Badge variant="secondary">{tag}</Badge></li>
                {/each}
              </ul>
            </Table.Cell>

            <Table.Cell class="align-top">
              <Switch
                checked={entry.rule.enabled}
                aria-label="Enable {entry.rule.name}"
                onCheckedChange={(enabled) => setEnabled(entry, enabled)}
              />
            </Table.Cell>

            <Table.Cell class="align-top">
              <div class="flex justify-end gap-1">
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label="Edit {entry.rule.name}"
                  onclick={() => onedit(entry)}
                >
                  <PencilIcon />
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label="Delete {entry.rule.name}"
                  onclick={() => (confirming = entry)}
                >
                  <Trash2Icon />
                </Button>
              </div>
            </Table.Cell>
          </Table.Row>
        {/each}
      </Table.Body>
    </Table.Root>
  </div>
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

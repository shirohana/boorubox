<script lang="ts">
  // Slot Settings · Stamps, the listing half (`stamps` design D5): text
  // rather than a pattern, and no enable switch — a stamp does nothing on its
  // own until it is made active in edit mode, unlike a rule that runs on
  // every arriving image. Ordered by creation, as `stampsList` (`created_at,
  // rowid`) already answers it (design D3); reordering is a non-goal.
  import type { Stamp } from '@boorubox/shared'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { errorText, stampsDelete } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Table from '$lib/components/ui/table'

  interface Props {
    stamps: Stamp[]
    onedit: (stamp: Stamp) => void
    /** A write was attempted: the caller re-reads the list, landed or not. */
    onchanged: () => void | Promise<void>
    onerror: (message: string) => void
  }

  let { stamps, onedit, onchanged, onerror }: Props = $props()

  let confirming = $state<Stamp | null>(null)

  function deleteConfirmed() {
    const doomed = confirming
    confirming = null
    if (doomed) void run(() => stampsDelete(doomed.id))
  }

  async function run(action: () => Promise<unknown>) {
    let failure: string | null = null
    try {
      await action()
    } catch (cause) {
      failure = errorText(cause)
    }
    await onchanged()
    if (failure !== null) onerror(failure)
  }
</script>

{#if stamps.length === 0}
  <p class="text-sm text-muted-foreground">
    No stamps yet. A stamp is a saved edit, written once in the tag language and applied to
    images by a click in edit mode, or to a selection at once.
  </p>
{:else}
  <div class="overflow-x-auto rounded-lg border border-border">
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head>Text</Table.Head>
          <Table.Head class="w-20"></Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each stamps as stamp (stamp.id)}
          <Table.Row>
            <Table.Cell class="align-top font-medium">{stamp.name}</Table.Cell>
            <Table.Cell class="align-top">
              <code class="font-mono text-xs break-all">{stamp.text}</code>
            </Table.Cell>
            <Table.Cell class="align-top">
              <div class="flex justify-end gap-1">
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label="Edit {stamp.name}"
                  onclick={() => onedit(stamp)}
                >
                  <PencilIcon />
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label="Delete {stamp.name}"
                  onclick={() => (confirming = stamp)}
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
  A stamp deleted mid-review is not recoverable, so this asks — `RulesTable`'s
  own dialog, the same reason.
-->
<ConfirmDialog
  title="Delete “{confirming?.name}”?"
  description="The stamp is gone and stops appearing in the bar. Nothing it has already
    applied to an image changes."
  confirmLabel="Delete stamp"
  open={confirming !== null}
  onclose={() => (confirming = null)}
  onconfirm={deleteConfirmed}
/>

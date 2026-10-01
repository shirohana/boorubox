<script lang="ts">
  // Slot Settings · Stamps, the listing half (`stamps` design D5): text
  // rather than a pattern, and no enable switch — a stamp does nothing on its
  // own until it is made active in edit mode, unlike a rule that runs on
  // every arriving image. Listed in the order `stampsList` answers (`stamp-order`
  // design D1). A move is written through `stampsReorder` and the list is
  // re-read afterwards: the store is the one order the table and the stamp bar
  // share, so the table never keeps a reordered copy of its own (`stamp-order`
  // design D6).
  import type { Stamp } from '@boorubox/shared'
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down'
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { errorText, stampsDelete, stampsReorder } from '$lib/api'
  import ConfirmDialog from '$lib/components/common/ConfirmDialog.svelte'
  import { moveItem, reorderable } from '$lib/components/common/reorder'
  import ReorderHandle from '$lib/components/common/ReorderHandle.svelte'
  import { Button } from '$lib/components/ui/button'
  import * as ContextMenu from '$lib/components/ui/context-menu'
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

  let reordering = false

  // A second move before the re-read would start from the old order and send it again.
  async function move(from: number, to: number) {
    const moved = moveItem(stamps, from, to)
    if (reordering || moved === stamps) return
    reordering = true
    try {
      await run(() => stampsReorder(moved.map((stamp) => stamp.id)))
    } finally {
      reordering = false
    }
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
  <div
    class="overflow-x-auto rounded-lg border border-border"
    use:reorderable={{ onmove: move }}
  >
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head class="w-8"></Table.Head>
          <Table.Head>Name</Table.Head>
          <Table.Head>Text</Table.Head>
          <Table.Head class="w-20"></Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each stamps as stamp, index (stamp.id)}
          <ContextMenu.Root>
            <ContextMenu.Trigger>
              {#snippet child({ props })}
                <Table.Row
                  {...props}
                  data-reorder-index={index}
                  class="
                    data-[reorder-drop=after]:*:shadow-[inset_0_-2px_0_var(--color-primary)]
                    data-[reorder-drop=before]:*:shadow-[inset_0_2px_0_var(--color-primary)]
                  "
                >
                  <Table.Cell class="w-8 align-top">
                    <ReorderHandle />
                  </Table.Cell>
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
              {/snippet}
            </ContextMenu.Trigger>
            <ContextMenu.Content>
              <ContextMenu.Item onSelect={() => onedit(stamp)}>
                <PencilIcon />
                Edit…
              </ContextMenu.Item>
              <ContextMenu.Item variant="destructive" onSelect={() => (confirming = stamp)}>
                <Trash2Icon />
                Delete…
              </ContextMenu.Item>
              <ContextMenu.Separator />
              <!--
                Disabled at the ends, not hidden: the item's place in the menu
                is what a repeat user's hand learns (`stamp-order` design D5).
              -->
              <ContextMenu.Item disabled={index === 0} onSelect={() => move(index, index - 1)}>
                <ArrowUpIcon />
                Move up
              </ContextMenu.Item>
              <ContextMenu.Item
                disabled={index === stamps.length - 1}
                onSelect={() => move(index, index + 1)}
              >
                <ArrowDownIcon />
                Move down
              </ContextMenu.Item>
            </ContextMenu.Content>
          </ContextMenu.Root>
        {/each}
      </Table.Body>
    </Table.Root>
  </div>
{/if}

<!--
  A stamp deleted mid-review is not recoverable, so this asks — `RuleList`'s
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

<script lang="ts">
  // Pinned groups, managed in one place (`pinned-group-management` design
  // D10): name a group, put the groups in order, move several tags at once,
  // start a new group, delete an empty one. Mounted unconditionally by each
  // host beside its menu (`TagNoteDialog`'s reason: a Dialog torn down while
  // open logs `derived_inert`). Reads and writes the vocabulary store, whose
  // every answer replaces the groups and their tags together, so the sections
  // below cannot show a name on the wrong row after a compaction.
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down'
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import { vocabulary } from '$lib/api'
  import { moveItem, reorderable } from '$lib/components/common/reorder'
  import ReorderHandle from '$lib/components/common/ReorderHandle.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Input } from '$lib/components/ui/input'
  import * as Select from '$lib/components/ui/select'
  import { KEY_ENTER } from '$lib/keyboard'
  import { CATEGORY_TEXT_CLASS } from './categories'

  interface Props {
    open: boolean
    /** Where this dialog portals: the viewer's `<dialog>` when opened from inside it. */
    portalTo?: Element
    /** Dismissed. Every change has already been written. */
    onclose: () => void
  }

  let { open, portalTo, onclose }: Props = $props()

  const NEW_GROUP = 'new'

  // `drafts` and `renameErrors` are keyed by position, so any write that can
  // renumber the groups (move, delete, a rename to blank) clears both.
  /** What was typed into a group's name field and not yet written, by position. */
  let drafts = $state<Record<number, string>>({})
  /** A refusal of a group's rename, by position. */
  let renameErrors = $state<Record<number, string>>({})
  /** Tags ticked for a move; a tag that stops being pinned drops out of {@link selected}. */
  let ticked = $state<string[]>([])
  /** The new-group prompt: `move` says whether the ticked tags go into the group it creates. */
  let naming = $state<{ move: boolean } | null>(null)
  let newName = $state('')
  let newError = $state<string | null>(null)
  /** The move Select's value, reset after each choice so choosing the same group again fires. */
  let moveChoice = $state('')

  const groups = $derived(vocabulary.pinnedGroups)
  const selected = $derived(ticked.filter((tag) => vocabulary.isPinned(tag)))
  /** The one group every selected tag already sits in, which a move into would do nothing. */
  const sharedGroup = $derived.by(() => {
    const positions = new Set(selected.map((tag) => vocabulary.groupOf(tag)))
    return positions.size === 1 ? [...positions][0] : null
  })
  const targets = $derived(
    groups.map((_, index) => index + 1).filter((position) => position !== sharedGroup),
  )

  $effect(() => {
    if (open) {
      drafts = {}
      renameErrors = {}
      ticked = []
      naming = null
    }
  })

  function nameOf(position: number): string {
    return drafts[position] ?? vocabulary.groups[position - 1]?.name ?? ''
  }

  async function commitName(position: number) {
    const draft = drafts[position]
    if (draft === undefined) return
    if (draft === vocabulary.groups[position - 1]?.name) {
      delete drafts[position]
      return
    }
    const landed = await vocabulary.renameGroup(position, draft)
    if (landed) {
      if (draft.trim() === '') clearPositionKeyed()
      delete drafts[position]
      delete renameErrors[position]
    } else {
      renameErrors[position] = vocabulary.error ?? 'The group could not be renamed'
    }
  }

  function clearPositionKeyed() {
    drafts = {}
    renameErrors = {}
  }

  async function deleteGroup(position: number) {
    await vocabulary.deleteGroup(position)
    clearPositionKeyed()
  }

  function moveGroup(from: number, to: number) {
    if (moveItem(groups, from, to) === groups) return
    clearPositionKeyed()
    void vocabulary.moveGroup(from + 1, to + 1)
  }

  function toggle(tag: string, on: boolean) {
    ticked = on ? [...ticked, tag] : ticked.filter((other) => other !== tag)
  }

  async function moveSelectedTo(value: string) {
    moveChoice = ''
    if (value === NEW_GROUP) {
      openNaming(true)
      return
    }
    await vocabulary.placeMany(selected, { group: Number(value) })
    if (!vocabulary.error) ticked = []
  }

  function openNaming(move: boolean) {
    naming = { move }
    newName = ''
    newError = null
  }

  async function createNamed() {
    if (!naming) return
    const names = naming.move ? selected : []
    if (!(await vocabulary.createGroup(newName))) {
      newError = vocabulary.error
      return
    }
    if (names.length > 0) {
      await vocabulary.placeMany(names, { group: vocabulary.groups.length })
      if (!vocabulary.error) ticked = []
    }
    naming = null
  }
</script>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) onclose() }}>
  <Dialog.Content portalProps={{ to: portalTo }} class="max-h-[85vh] overflow-y-auto sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>Pinned groups</Dialog.Title>
      <Dialog.Description>
        Name the groups, drag them into order, and move several tags between them at once.
      </Dialog.Description>
    </Dialog.Header>

    <div class="flex flex-col gap-3" use:reorderable={{ onmove: moveGroup }}>
      {#each groups as group, index (index)}
        {@const position = index + 1}
        <ContextMenu.Root>
          <ContextMenu.Trigger>
            {#snippet child({ props })}
              <section
                {...props}
                aria-label="Group {vocabulary.labelOf(position)}"
                data-reorder-index={index}
                class="
                  flex flex-col gap-1.5 rounded-md border border-border p-2
                  data-[reorder-drop=after]:shadow-[inset_0_-2px_0_var(--color-primary)]
                  data-[reorder-drop=before]:shadow-[inset_0_2px_0_var(--color-primary)]
                "
              >
                <div class="flex items-center gap-1.5">
                  <ReorderHandle />
                  <Input
                    value={nameOf(position)}
                    autocomplete="off"
                    placeholder="#{position}"
                    aria-label="Name of group {position}"
                    aria-invalid={renameErrors[position] ? true : undefined}
                    oninput={(event) => (drafts[position] = event.currentTarget.value)}
                    onblur={() => void commitName(position)}
                    onkeydown={(event) => {
                      if (event.key !== KEY_ENTER) return
                      event.preventDefault()
                      void commitName(position)
                    }}
                  />
                </div>
                {#if renameErrors[position]}
                  <p class="text-xs text-destructive">{renameErrors[position]}</p>
                {/if}
                {#if group.tags.length === 0}
                  <p class="px-1 text-xs text-muted-foreground">No tags</p>
                {:else}
                  <ul role="list" class="flex flex-col gap-1">
                    {#each group.tags as tag (tag)}
                      <li>
                        <label class="flex items-center gap-2 px-1 text-xs">
                          <Checkbox
                            checked={selected.includes(tag)}
                            onCheckedChange={(on) => toggle(tag, on)}
                            aria-label="Select {tag}"
                          />
                          <span class={CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]}>{tag}</span>
                        </label>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </section>
            {/snippet}
          </ContextMenu.Trigger>
          <ContextMenu.Content portalProps={{ to: portalTo }}>
            <ContextMenu.Item disabled={index === 0} onSelect={() => moveGroup(index, index - 1)}>
              <ArrowUpIcon />
              Move up
            </ContextMenu.Item>
            <ContextMenu.Item
              disabled={index === groups.length - 1}
              onSelect={() => moveGroup(index, index + 1)}
            >
              <ArrowDownIcon />
              Move down
            </ContextMenu.Item>
            {#if group.tags.length === 0}
              <ContextMenu.Separator />
              <ContextMenu.Item
                variant="destructive"
                onSelect={() => void deleteGroup(position)}
              >
                <Trash2Icon />
                Delete group
              </ContextMenu.Item>
            {/if}
          </ContextMenu.Content>
        </ContextMenu.Root>
      {/each}
    </div>

    {#if vocabulary.error}
      <p class="text-xs text-destructive">{vocabulary.error}</p>
    {/if}

    {#if naming}
      <form
        class="flex flex-col gap-2"
        onsubmit={(event) => {
          event.preventDefault()
          void createNamed()
        }}
      >
        <Input
          bind:value={newName}
          autocomplete="off"
          autofocus
          placeholder="Group name"
          aria-label="New group name"
        />
        {#if newError}
          <p class="text-xs text-destructive">{newError}</p>
        {/if}
        <div class="flex justify-end gap-2">
          <Button type="button" variant="ghost" onclick={() => (naming = null)}>Cancel</Button>
          <Button type="submit">
            {naming.move ? `Create and move ${selected.length}` : 'Create'}
          </Button>
        </div>
      </form>
    {:else}
      <Dialog.Footer>
        {#if selected.length > 0}
          <Select.Root type="single" bind:value={moveChoice} onValueChange={(value) => void moveSelectedTo(value)}>
            <Select.Trigger aria-label="Move selected tags to" class="sm:mr-auto">
              Move {selected.length} selected to…
            </Select.Trigger>
            <Select.Content portalProps={{ to: portalTo }}>
              {#each targets as position (position)}
                <Select.Item value={String(position)} label={vocabulary.labelOf(position)} />
              {/each}
              <Select.Item value={NEW_GROUP} label="New group…" />
            </Select.Content>
          </Select.Root>
        {/if}
        <Button type="button" variant="outline" onclick={() => openNaming(false)}>
          New group…
        </Button>
        <Button type="button" onclick={onclose}>Done</Button>
      </Dialog.Footer>
    {/if}
  </Dialog.Content>
</Dialog.Root>

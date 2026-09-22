<script lang="ts">
  // The bulk tag edit (design D8). A dialog because the operation is two
  // multi-value fields and a confirm, which is not a toolbar's shape.
  import type { TagCount } from '@boorubox/shared'
  import type { Selection } from '$lib/api'
  import { bulkUpdateTags, errorText, selectionTagCounts } from '$lib/api'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { tagList } from '$lib/domain/tag-utils'

  /** The legacy modal's number, and the ten of spec `bulk-operations`. */
  const PILL_LIMIT = 10
  /**
   * The pool the remove field suggests from — deeper than the pills, because
   * that field is typed into. One query answers both (design D9).
   */
  const TAG_FETCH = 200

  interface Props {
    selection: Selection
    /**
     * Called once the write lands (`browse-fixes` design D1): the screen's
     * `afterWrite`, which re-reads the search, keeps the selection to what it
     * still matches, and puts the focus back on a row that exists. This
     * dialog no longer refreshes on its own — a bulk tag edit is one of
     * several writes that have to prune the same way, and a second copy of
     * that rule here is a second place for it to drift from the others'.
     */
    onapplied: () => Promise<void>
    open: boolean
  }

  let { selection, onapplied, open = $bindable() }: Props = $props()

  let counts = $state<TagCount[]>([])
  let add = $state('')
  let remove = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  const pills = $derived(counts.slice(0, PILL_LIMIT))
  const removing = $derived(tagList(remove))
  const nothingToDo = $derived(tagList(add).length === 0 && removing.length === 0)

  // Opening is an action, so it resolves the selection to ids (design D3) and
  // asks Rust for the frequencies. `asked` guards the effect from re-running on
  // its own writes; it is reset when the dialog closes, so the next open counts
  // whatever is selected then.
  let asked = false
  $effect(() => {
    if (!open) {
      asked = false
      return
    }
    if (asked) return
    asked = true
    void load()
  })

  function toggleInList(text: string, tag: string): string {
    const tags = tagList(text)
    const left = tags.filter((other) => other !== tag)
    return (left.length === tags.length ? [...tags, tag] : left).join(' ')
  }

  async function load() {
    counts = []
    error = null
    try {
      counts = await selectionTagCounts(await selection.ids(), TAG_FETCH)
    } catch (cause) {
      error = errorText(cause)
    }
  }

  /**
   * Design D8: the remove field offers only what the selection actually
   * carries. The whole pool is already in hand, so the prefix is matched here
   * rather than in a second round trip per keystroke.
   */
  function selectionTags(prefix: string, limit: number): Promise<string[]> {
    const typed = prefix.toLowerCase()
    return Promise.resolve(
      counts
        .filter((tag) => tag.name.toLowerCase().startsWith(typed))
        .slice(0, limit)
        .map((tag) => tag.name),
    )
  }

  async function apply() {
    if (saving) return
    saving = true
    error = null
    try {
      await bulkUpdateTags(await selection.ids(), tagList(add), tagList(remove))
      // The edit is on rows the grid and the inspector are drawing; a refresh
      // is the same list, so it keeps its scroll (design D4). Closed before
      // the await: `onapplied` prunes the selection, and a trigger that the
      // prune unmounts must already be gone before bits-ui's close-auto-focus
      // goes looking for it.
      add = ''
      remove = ''
      open = false
      await onapplied()
    } catch (cause) {
      // The fields keep what was typed: the edit did not happen, and retyping
      // it is the last thing anyone wants to do after being told so.
      error = errorText(cause)
    } finally {
      saving = false
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>Tag {selection.count.toLocaleString()} images</Dialog.Title>
      <Dialog.Description>
        Every selected image is edited in one write, or none of them is.
      </Dialog.Description>
    </Dialog.Header>

    <!--
      Headings, not `<label for>`: a label activates its field on click, and the
      dead space between these two fields is exactly where someone reaches to
      click a suggestion or a pill, which reopened the suggestion list under
      their pointer. Each field carries its own name in `aria-label`, so nothing
      is lost by the association going.
    -->
    <div class="grid gap-1.5">
      <h3 class="text-xs font-medium text-muted-foreground">Add tags</h3>
      <TagInput
        bind:value={add}
        label="Tags to add to every selected image"
        placeholder="Tags, separated by spaces"
        class="h-8"
      />
    </div>

    <div class="grid gap-1.5">
      <h3 class="text-xs font-medium text-muted-foreground">Remove tags</h3>
      <TagInput
        bind:value={remove}
        label="Tags to remove from every selected image"
        placeholder="Tags, separated by spaces"
        class="h-8"
        suggest={selectionTags}
      />

      {#if pills.length > 0}
        <!--
          The commonest tags in the selection with the number of selected images
          carrying each: counted in Rust over the whole selection, because a
          count from the loaded pages would be confidently wrong (design D9).
        -->
        <ul class="mt-1 flex flex-wrap gap-1">
          {#each pills as tag (tag.name)}
            <li>
              <button type="button" onclick={() => (remove = toggleInList(remove, tag.name))}>
                <Badge variant={removing.includes(tag.name) ? 'default' : 'secondary'}>
                  {tag.name}
                  <span class="tabular-nums opacity-70">{tag.count.toLocaleString()}</span>
                </Badge>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    {#if error}
      <p class="text-xs text-destructive">{error}</p>
    {/if}

    <Dialog.Footer>
      <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
      <Button disabled={saving || nothingToDo} onclick={apply}>Apply</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

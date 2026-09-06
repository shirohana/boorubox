<script lang="ts">
  // One component, two placements (design D9): the library route's right column
  // and the lightbox's inspect mode. Both edit — the tag editor and the rating
  // control are the same instances in both, which is the point of there being
  // one component (slots Inspector · tags and Inspector · rating, design D17).
  import type { ImageRecord } from '@boorubox/shared'
  import type { SearchResults } from '$lib/api'
  import { errorText } from '$lib/api'
  import RatingControl from '$lib/components/tags/RatingControl.svelte'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { formatBytes, formatTimestamp } from '$lib/domain/format'
  import { excludeTagFromQuery, sortTags, toggleTagInQuery } from '$lib/domain/tag-utils'

  interface Props {
    image: ImageRecord | null
    /** Where an edit is written and the changed record is put back (design D10). */
    results: SearchResults
    /** The toolbar's tag query, so a click on a tag can rewrite it (design D14). */
    tagQuery: string
    onquery: (next: string) => void
  }

  let { image, results, tagQuery, onquery }: Props = $props()

  const title = $derived(image?.pageTitle || image?.imageUrl || image?.id || '')
  const origin = $derived(
    image ? (image.sourceRef ? `${image.source} · ${image.sourceRef}` : image.source) : '',
  )
  /** `sortTags` is the only tag order, applied at render (design D4). */
  const tags = $derived(image ? sortTags(image.tags) : [])
  const saved = $derived(tags.join(' '))

  let tagInput = $state<TagInput | null>(null)
  let draft = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)
  const dirty = $derived(draft.trim() !== saved)

  // The editor follows the record: another image, or the same one after a write.
  // `updatedAt` is what a save moves, so the text comes back sorted from the row
  // that was stored rather than from what was typed (design D4).
  let shown = ''
  $effect(() => {
    const key = image ? `${image.id}:${image.updatedAt}` : ''
    if (key === shown) return
    shown = key
    draft = saved
    error = null
  })

  async function write(tags: string[]) {
    if (!image || saving) return
    saving = true
    error = null
    try {
      await results.saveTags(image.id, tags)
    } catch (cause) {
      error = errorText(cause)
    } finally {
      saving = false
    }
  }

  /** Design D2: the editor holds the whole set, so the whole set is sent. */
  const save = () => write(draft.split(/\s+/).filter((tag) => tag.length > 0))

  const remove = (tag: string) => write(tags.filter((other) => other !== tag))

  /**
   * A keyboard confirmation in the editor saves and then, once the write has
   * gone through, blurs it so the grid's keyboard map is live again. The blur is
   * conditional on success: a failed save keeps the focus so the text can be
   * fixed and confirmed again without reaching for the field.
   */
  async function submitFromEditor() {
    await save()
    if (!error) tagInput?.blur()
  }
</script>

<div class="flex h-full flex-col overflow-y-auto">
  {#if !image}
    <p class="p-4 text-sm text-muted-foreground">No image selected</p>
  {:else}
    <header class="border-b border-border px-4 py-3">
      <h2 class="text-sm font-medium wrap-break-word">{title}</h2>
    </header>

    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 px-4 py-3 text-xs">
      <!--
        The header falls back to the image URL and then the id so it is never
        blank; this row is the stored page title itself, which is often absent.
      -->
      <dt class="text-muted-foreground">Title</dt>
      <dd class="wrap-break-word">{image.pageTitle ?? '—'}</dd>

      <dt class="text-muted-foreground">Source</dt>
      <dd class="wrap-break-word">{origin}</dd>

      <dt class="text-muted-foreground">Page</dt>
      <dd class="wrap-break-word">{image.pageUrl ?? '—'}</dd>

      <dt class="text-muted-foreground">Image</dt>
      <dd class="wrap-break-word">{image.imageUrl ?? '—'}</dd>

      <dt class="text-muted-foreground">Dimensions</dt>
      <dd>{image.width} × {image.height}</dd>

      <dt class="text-muted-foreground">Size</dt>
      <dd>{formatBytes(image.size)}</dd>

      <dt class="text-muted-foreground">Type</dt>
      <dd>{image.mime}</dd>

      <dt class="text-muted-foreground">Captured</dt>
      <dd>{formatTimestamp(image.capturedAt)}</dd>

      <dt class="text-muted-foreground">Imported</dt>
      <dd>{formatTimestamp(image.createdAt)}</dd>

      <dt class="text-muted-foreground">ID</dt>
      <dd class="font-mono wrap-break-word">{image.id}</dd>
    </dl>

    <section class="border-t border-border px-4 py-3">
      <h3 class="mb-2 text-xs font-medium text-muted-foreground">Rating</h3>
      <RatingControl {image} {results} />
    </section>

    <section class="border-t border-border px-4 py-3">
      <h3 class="mb-2 text-xs font-medium text-muted-foreground">
        Tags {#if tags.length > 0}({tags.length}){/if}
      </h3>

      <div class="flex items-center gap-2">
        <TagInput
          bind:this={tagInput}
          bind:value={draft}
          label="Tags of this image"
          placeholder="Tags, separated by spaces"
          class="h-8 min-w-0 flex-1"
          onsubmit={submitFromEditor}
        />
        {#if dirty}
          <Button size="xs" disabled={saving} onclick={save}>Save</Button>
        {/if}
      </div>

      {#if error}
        <p class="mt-2 text-xs text-destructive">{error}</p>
      {/if}

      {#if tags.length === 0}
        <p class="mt-2 text-xs text-muted-foreground">No tags</p>
      {:else}
        <!--
          Every tag on screen is a search term (spec `tag-editing`): a click puts
          it in the query or takes it out, and the menu offers the other two
          things one can do to a tag.
        -->
        <ul class="mt-2 flex flex-wrap gap-1">
          {#each tags as tag (tag)}
            <li>
              <ContextMenu.Root>
                <ContextMenu.Trigger>
                  {#snippet child({ props })}
                    <button
                      type="button"
                      {...props}
                      onclick={() => onquery(toggleTagInQuery(tagQuery, tag))}
                    >
                      <Badge variant="secondary">{tag}</Badge>
                    </button>
                  {/snippet}
                </ContextMenu.Trigger>
                <ContextMenu.Content>
                  <ContextMenu.Item onSelect={() => onquery(toggleTagInQuery(tagQuery, tag))}>
                    Search for this tag
                  </ContextMenu.Item>
                  <ContextMenu.Item onSelect={() => onquery(excludeTagFromQuery(tagQuery, tag))}>
                    Exclude from the search
                  </ContextMenu.Item>
                  <ContextMenu.Separator />
                  <ContextMenu.Item variant="destructive" onSelect={() => remove(tag)}>
                    Remove from this image
                  </ContextMenu.Item>
                </ContextMenu.Content>
              </ContextMenu.Root>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

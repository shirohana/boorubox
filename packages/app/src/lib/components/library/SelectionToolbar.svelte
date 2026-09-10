<script lang="ts">
  // Slot Toolbar · actions (design D6): while a selection exists this row is the
  // selection's, and only this row. The search and the view controls stay where
  // they are — a selection is a thing you have, not a mode you are in, and it is
  // one `Esc` away from over.
  //
  // Icons rather than words (design D6, amended): the row of labelled buttons
  // this shipped as was wider than the window it had to share with the search
  // fields, the view controls and the tile slider, and pushed the whole top bar
  // into a horizontal scroll. Each button keeps its words in `aria-label` and in
  // its tooltip, and shows them again on a window wide enough to hold them.
  import type { ExportProgress, ExportReport, Rating } from '@boorubox/shared'
  import type { Component } from 'svelte'
  import ArchiveRestoreIcon from '@lucide/svelte/icons/archive-restore'
  import FileArchiveIcon from '@lucide/svelte/icons/file-archive'
  import ListChecksIcon from '@lucide/svelte/icons/list-checks'
  import TagsIcon from '@lucide/svelte/icons/tags'
  import Trash2Icon from '@lucide/svelte/icons/trash-2'
  import XIcon from '@lucide/svelte/icons/x'
  import type { SearchResults, Selection } from '$lib/api'
  import { bulkSetRating, errorText, exportZip, onExportProgress, pickExportZipPath } from '$lib/api'
  import RatingControl from '$lib/components/tags/RatingControl.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Progress } from '$lib/components/ui/progress'
  import BulkTagDialog from './BulkTagDialog.svelte'
  import type { TrashActions } from './trash-actions'

  interface Props {
    selection: Selection
    results: SearchResults
    /**
     * Slot Toolbar · actions (`trash` design D13): the same pair every other
     * control offers, over the selection instead of one image. The screen owns
     * what a write refreshes, so this row only resolves the ids and calls it.
     */
    actions: TrashActions
    /** Reported by the page, beside its other action failures. */
    onerror: (message: string) => void
    /** The report outlives this row, so the page holds it (design D11). */
    onexported: (report: ExportReport) => void
  }

  let { selection, results, actions, onerror, onexported }: Props = $props()

  let tagsOpen = $state(false)
  let exporting = $state(false)
  let progress = $state<ExportProgress | null>(null)

  // Design D13: the ticks arrive whether or not this row asked for them, so the
  // subscription is the row's whole life rather than the export's.
  $effect(() => {
    const subscription = onExportProgress((tick) => (progress = tick))
    subscription.catch((cause) => onerror(errorText(cause)))
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  /**
   * One write over the whole selection (design D10), then the same search
   * again: the rows are edited, and by now the selection is ids, so it survives
   * the refresh (design D4). A failure is shown by the control itself.
   */
  async function rate(rating: Rating | null) {
    await bulkSetRating(await selection.ids(), rating)
    await results.refresh()
  }

  /**
   * A range selection spans rows the app has never loaded, so the ids come from
   * Rust before the action can name them (`selection-and-bulk` design D3). One
   * call for the whole selection, one write.
   */
  async function apply(run: (ids: string[]) => void) {
    try {
      run(await selection.ids())
    } catch (cause) {
      onerror(errorText(cause))
    }
  }

  async function exportSelected() {
    if (exporting) return
    try {
      const path = await pickExportZipPath()
      // Cancelling writes nothing — including no `export_zip` call.
      if (path === null) return
      const ids = await selection.ids()
      exporting = true
      progress = { done: 0, total: ids.length }
      onexported(await exportZip(ids, path))
    } catch (cause) {
      onerror(errorText(cause))
    } finally {
      exporting = false
      progress = null
    }
  }
</script>

<!--
  `min-w-0` and the clip: whatever is in this row, it can never make the top bar
  wider than the window. The count comes first because it is the fact the rest
  of the row is about.
-->
<!--
  `overflow-x-auto`, never hidden: the search fields keep a floor and the view
  controls never shrink, so this row is what gives at a narrow window and is
  cut from the right — with a scrollbar the last action is a swipe away,
  clipped it is unreachable.
-->
<div class="flex min-w-0 items-center gap-1 overflow-x-auto">
  <span class="shrink-0 px-1 text-xs text-muted-foreground tabular-nums">
    {selection.count.toLocaleString()}<span class="hidden 2xl:inline"> selected</span>
  </span>

  {@render action(ListChecksIcon, 'Select all', 'ghost', false, () =>
    selection.selectAll(results.total))}
  {@render action(XIcon, 'Clear', 'ghost', false, () => selection.clear())}
  {@render action(TagsIcon, 'Tags…', 'outline', false, () => (tagsOpen = true))}

  <!-- No single current rating to show: this sets one, it does not report one. -->
  <RatingControl value={undefined} onchoose={rate} />

  {@render action(FileArchiveIcon, 'Export…', 'outline', exporting, exportSelected)}

  {#if results.view === 'trash'}
    {@render action(ArchiveRestoreIcon, 'Restore', 'outline', false, () => apply(actions.restore))}
    {@render action(Trash2Icon, 'Delete forever…', 'destructive', false, () =>
      apply(actions.deleteForever))}
  {:else}
    {@render action(Trash2Icon, 'Move to trash', 'outline', false, () => apply(actions.trash))}
  {/if}

  {#if progress}
    <Progress value={progress.done} max={progress.total} class="w-20 shrink-0" />
    <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
      {progress.done.toLocaleString()} / {progress.total.toLocaleString()}
    </span>
  {/if}
</div>

<!--
  One shape for every action in the row, so a button cannot be added without its
  name: the label is the accessible name, the tooltip, and the text the row
  shows once the window is wide enough for all of them.
-->
{#snippet action(
  Icon: Component,
  label: string,
  variant: 'ghost' | 'outline' | 'destructive',
  disabled: boolean,
  run: () => void,
)}
  <Button
    size="sm"
    {variant}
    {disabled}
    class="shrink-0"
    aria-label={label}
    title={label}
    onclick={run}
  >
    <Icon />
    <span class="hidden 2xl:inline">{label}</span>
  </Button>
{/snippet}

<BulkTagDialog {selection} {results} bind:open={tagsOpen} />

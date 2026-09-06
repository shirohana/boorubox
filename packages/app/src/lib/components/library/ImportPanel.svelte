<script lang="ts">
  import type { ImportProgress, ImportReport } from '@boorubox/shared'
  import {
    errorText,
    importPaths,
    onFileDrop,
    onImportProgress,
    pickImportFiles,
    pickImportFolder,
  } from '$lib/api'
  import { Button } from '$lib/components/ui/button'

  let { onimported }: { onimported: () => void } = $props()

  let hovering = $state(false)
  let running = $state(false)
  /** null until the first `import:progress` event; Rust counts the files first. */
  let progress = $state<ImportProgress | null>(null)
  let report = $state<ImportReport | null>(null)
  let error = $state<string | null>(null)

  const unimported = $derived(report?.items.filter((item) => item.status !== 'imported') ?? [])

  $effect(() => {
    const subscription = onFileDrop({
      onhover: () => (hovering = true),
      onleave: () => (hovering = false),
      ondrop: (paths) => {
        hovering = false
        void run(paths)
      },
    })
    subscription.catch((cause) => (error = errorText(cause)))
    return () => {
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  })

  async function run(paths: string[]) {
    if (running || paths.length === 0) return
    running = true
    report = null
    error = null
    progress = null

    // Subscribed before the command starts, or the first events of a fast
    // import are lost and the count jumps.
    const unlisten = await onImportProgress((update) => (progress = update))
    try {
      report = await importPaths(paths)
      onimported()
    } catch (cause) {
      error = errorText(cause)
    } finally {
      await unlisten()
      running = false
      progress = null
    }
  }

  async function pick(picker: () => Promise<string[]>) {
    try {
      await run(await picker())
    } catch (cause) {
      error = errorText(cause)
    }
  }
</script>

<div class="flex flex-col gap-2">
  <div class="flex flex-wrap items-center gap-2">
    <Button
      size="sm"
      variant="outline"
      disabled={running}
      onclick={() => pick(pickImportFiles)}
    >
      Import files…
    </Button>
    <Button
      size="sm"
      variant="outline"
      disabled={running}
      onclick={() => pick(pickImportFolder)}
    >
      Import folder…
    </Button>
    <span class="text-xs text-muted-foreground">or drop files and folders on this window</span>
  </div>

  {#if running}
    <p role="status" class="text-xs text-muted-foreground">
      {#if progress}
        {progress.imported.toLocaleString()} imported ·
        {progress.done.toLocaleString()} of {progress.total.toLocaleString()} files
      {:else}
        Looking through what you dropped…
      {/if}
    </p>
  {/if}

  {#if error}
    <p class="text-xs text-destructive">{error}</p>
  {/if}

  {#if report}
    <div class="rounded-lg border border-border p-2 text-xs">
      <div class="flex items-baseline justify-between gap-2">
        <p>
          Imported {report.imported.toLocaleString()} · skipped
          {report.skipped.toLocaleString()} · failed {report.failed.toLocaleString()}
        </p>
        <Button size="xs" variant="ghost" onclick={() => (report = null)}>Dismiss</Button>
      </div>
      {#if unimported.length > 0}
        <ul class="mt-1 max-h-32 overflow-y-auto text-muted-foreground">
          {#each unimported as item (item.path)}
            <li class="break-all">
              <span class="font-medium">{item.status}</span>
              {item.path}{item.reason ? ` — ${item.reason}` : ''}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>

{#if hovering}
  <div
    class="
      pointer-events-none fixed inset-0 z-40 flex items-center justify-center bg-background/80
    "
  >
    <p class="rounded-xl border border-dashed border-border px-6 py-4 text-sm">
      Drop images or folders to import them
    </p>
  </div>
{/if}

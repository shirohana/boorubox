<script lang="ts">
  // The toolbar's actions slot (design D8): two buttons in the old header row
  // became one menu, and the progress that used to sit under them now reads out
  // beside it, because a menu closes over its own progress line.
  import FileIcon from '@lucide/svelte/icons/file'
  import FolderIcon from '@lucide/svelte/icons/folder'
  import UploadIcon from '@lucide/svelte/icons/upload'
  import { pickImportFiles, pickImportFolder } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import type { Imports } from './imports.svelte'

  let { imports }: { imports: Imports } = $props()
</script>

<div class="flex items-center gap-2">
  {#if imports.running}
    <p role="status" class="text-xs text-muted-foreground tabular-nums">
      {#if imports.progress}
        {imports.progress.imported.toLocaleString()} imported ·
        {imports.progress.done.toLocaleString()} of {imports.progress.total.toLocaleString()}
      {:else}
        Looking through what you dropped…
      {/if}
    </p>
  {/if}

  <DropdownMenu.Root>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button size="sm" variant="outline" disabled={imports.running} {...props}>
          <UploadIcon />
          Import
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content align="end" class="w-48">
      <DropdownMenu.Item onSelect={() => imports.pick(pickImportFiles)}>
        <FileIcon />
        Import files…
      </DropdownMenu.Item>
      <DropdownMenu.Item onSelect={() => imports.pick(pickImportFolder)}>
        <FolderIcon />
        Import folder…
      </DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
</div>

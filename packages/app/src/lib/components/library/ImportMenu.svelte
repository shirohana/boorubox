<script lang="ts">
  // The toolbar's actions slot (design D8): the two buttons of the old header
  // row became one menu. It carries no progress and is never disabled — imports
  // queue, so the button has to stay available, and the band above the grid is
  // the one place a run's progress reads out (design D11).
  import FileIcon from '@lucide/svelte/icons/file'
  import FolderIcon from '@lucide/svelte/icons/folder'
  import UploadIcon from '@lucide/svelte/icons/upload'
  import { imports, pickImportFiles, pickImportFolder } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button size="sm" variant="outline" {...props}>
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

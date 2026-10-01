<script lang="ts">
  // Slot Settings · Pinned tags (`pinned-tags-panel` design D2): the panel the
  // inspector's "Manage pinned tags…" dialog also holds, after Stamps.
  import PinnedTagsPanel from '$lib/components/tags/PinnedTagsPanel.svelte'
</script>

<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">Pinned tags</h2>

  <p class="text-sm text-muted-foreground">
    A pinned tag is a one-click chip in the inspector, applied to the selected images or removed
    from them. Pinned tags are kept in groups, one row each; the groups are stored with the
    library, so copying the folder carries them.
  </p>

  <PinnedTagsPanel />
</section>

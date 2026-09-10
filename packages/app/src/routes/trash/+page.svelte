<script lang="ts">
  // The same screen over the trashed rows (`trash` design D1). A route rather
  // than a toggle on `/`: the sidebar's nav is a list of routes, and the view
  // has to survive a reload and a back gesture.
  import LibraryScreen from '$lib/components/library/LibraryScreen.svelte'
</script>

<LibraryScreen view="trash" />

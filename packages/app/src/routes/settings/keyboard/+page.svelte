<script lang="ts">
  import { Kbd } from '$lib/components/ui/kbd'
  import { KEYBOARD_MAP } from '$lib/keyboard'
</script>

<!-- The map is read-only here; every binding fires where it acts (design D14). -->
<section class="flex flex-col gap-4">
  <h2 class="text-sm font-semibold">Keyboard</h2>
  <dl class="grid grid-cols-[auto_auto_minmax(0,1fr)] items-baseline gap-x-6 gap-y-2 text-sm">
    {#each KEYBOARD_MAP as binding (binding.where + binding.action)}
      <dt class="text-muted-foreground">{binding.where}</dt>
      <dd class="flex flex-wrap gap-1">
        {#each binding.keys as key (key)}
          <Kbd>{key}</Kbd>
        {/each}
      </dd>
      <dd>{binding.action}</dd>
    {/each}
  </dl>
</section>

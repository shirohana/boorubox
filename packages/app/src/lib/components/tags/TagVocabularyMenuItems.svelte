<script lang="ts">
  // The tag context menu's vocabulary items — pin/unpin and the category
  // group — mounted wherever a tag's context menu opens (`tag-vocabulary`
  // design D9): the sidebar row, the inspector badge's menu, and the pinned
  // chip's own menu. A chip's tag is pinned by definition, so its menu shows
  // Unpin without this component needing a flag to suppress Pin.
  //
  // Snippet-free, unlike `CollectionMenuItems`: every mount point here is a
  // `ContextMenu.Root` (never a dropdown), so this renders `ContextMenu.*`
  // primitives directly rather than taking them as snippets.
  import { vocabulary } from '$lib/api'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { CATEGORY_ORDER, categoryLabel } from '$lib/domain/tag-categories'

  interface Props {
    name: string
  }

  let { name }: Props = $props()

  const pinned = $derived(vocabulary.isPinned(name))
  const category = $derived(vocabulary.categoryOf(name))
</script>

<ContextMenu.Item onSelect={() => void vocabulary.setPinned(name, !pinned)}>
  {pinned ? 'Unpin' : 'Pin'}
</ContextMenu.Item>
<ContextMenu.Separator />
<!--
  `ContextMenu.GroupHeading` reads a `Menu.Group` context and throws without
  one (bits-ui 2.19, `ImageCard`'s own rating group note).
-->
<ContextMenu.Group>
  <ContextMenu.GroupHeading>Category</ContextMenu.GroupHeading>
  {#each CATEGORY_ORDER as option (option)}
    <ContextMenu.CheckboxItem
      checked={category === option}
      onCheckedChange={() => {
        // Re-picking the current category is not a change: without this
        // guard it would still rewrite `library.json` for nothing.
        if (option === category) return
        void vocabulary.setCategory(name, option)
      }}
    >
      {categoryLabel(option)}
    </ContextMenu.CheckboxItem>
  {/each}
</ContextMenu.Group>

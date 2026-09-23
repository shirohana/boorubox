<script lang="ts">
  // The collection context menu's one pin/unpin item (`pinned-collections`
  // design D6), the collection-keyed twin of `TagVocabularyMenuItems`' Pin
  // item — mounted wherever a collection's context menu opens: the sidebar's
  // Collections section, the inspector's collection badge menu, and the
  // pinned chip's own menu, where `collection.pinned` is always true by
  // construction, so the item always reads Unpin there.
  //
  // Snippet-free, `TagVocabularyMenuItems`' own reason: every mount point
  // here is a `ContextMenu.Root`.
  import type { Collection } from '@boorubox/shared'
  import { collections } from '$lib/api'
  import * as ContextMenu from '$lib/components/ui/context-menu'

  interface Props {
    collection: Collection
  }

  let { collection }: Props = $props()
</script>

<ContextMenu.Item onSelect={() => void collections.setPinned(collection.id, !collection.pinned)}>
  {collection.pinned ? 'Unpin' : 'Pin'}
</ContextMenu.Item>

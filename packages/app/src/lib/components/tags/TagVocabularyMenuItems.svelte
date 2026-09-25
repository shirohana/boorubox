<script lang="ts">
  // The tag context menu's vocabulary items — a Danbooru look-up, pin/unpin,
  // a pinned tag's own group moves, and the category group — mounted
  // wherever a tag's context menu opens (`tag-vocabulary` design D9): the
  // sidebar row, the inspector badge's menu, and the pinned chip's own menu.
  // A chip's tag is pinned by definition, so its menu shows Unpin and the
  // group moves without this component needing a flag to suppress Pin.
  //
  // The group moves live here rather than on the chip itself
  // (`pinned-tag-groups` design D6) so a pinned tag offers the same moves
  // from every menu it has, sidebar and badge included, not only the chip.
  //
  // Snippet-free, unlike `CollectionMenuItems`: every mount point here is a
  // `ContextMenu.Root` (never a dropdown), so this renders `ContextMenu.*`
  // primitives directly rather than taking them as snippets.
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link'
  import PinIcon from '@lucide/svelte/icons/pin'
  import PinOffIcon from '@lucide/svelte/icons/pin-off'
  import { openExternal, vocabulary } from '$lib/api'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { CATEGORY_ORDER, categoryLabel } from '$lib/domain/tag-categories'
  import { danbooruLookup } from '$lib/domain/danbooru'
  import { CATEGORY_ICON } from './categories'

  interface Props {
    name: string
  }

  let { name }: Props = $props()

  const group = $derived(vocabulary.groupOf(name))
  const pinned = $derived(group !== null)
  const category = $derived(vocabulary.categoryOf(name))
  const lookup = $derived(danbooruLookup(name, category))

  /**
   * Every group but the tag's own, in order — the "Move to #x" items. A
   * counted range `1..groupCount` is the live set of groups only because
   * Rust keeps them dense after every write and every rebuild
   * (`pinned-tag-groups` design D3): group n is row n. It has to be the
   * live set — `Group(n)` clamps a number past the last group to a new last
   * group rather than refusing it, so an offered number that does not exist
   * would silently do something else.
   */
  const otherGroups = $derived(
    Array.from({ length: vocabulary.groupCount }, (_, i) => i + 1).filter((n) => n !== group),
  )
  /** The only tag of its group: inserting a group beside it changes nothing. */
  const alone = $derived(group !== null && vocabulary.pinnedGroups[group - 1]?.length === 1)
</script>

<ContextMenu.Item
  onSelect={() => {
    // FIXME(menu-polish D2): the failure is dropped rather than shown — a
    // menu that has closed has no row to show it in, and the URL built here
    // is always `https:`, never one of `ExternalLink`'s two named failures
    // (a `file:` address, a malformed one). Wants a shared transient-notice
    // surface the frame does not have yet.
    void openExternal(lookup.url)
  }}
>
  <ExternalLinkIcon />
  {lookup.label}
</ContextMenu.Item>
<ContextMenu.Separator />
<ContextMenu.Item onSelect={() => void vocabulary.place(name, pinned ? 'unpin' : { group: 1 })}>
  {#if pinned}
    <PinOffIcon />
  {:else}
    <PinIcon />
  {/if}
  {pinned ? 'Unpin' : 'Pin'}
</ContextMenu.Item>
{#if group !== null}
  <!--
    Inserting is the only operation the owner asked for (proposal, "no move
    up or move down"): "New group above"/"below" is `NewGroupAt`, "Move to
    #x" is `Group(x)` (`pinned-tag-groups` design D3). Not for a tag alone in
    its group: the new group would hold only this tag and the emptied one
    would close, leaving the strip as it was — a control that does nothing
    is not shown (`app-frame`, owner 2026-09-24).
  -->
  {#if !alone}
    <ContextMenu.Item onSelect={() => void vocabulary.place(name, { newGroupAt: group })}>
      New group above
    </ContextMenu.Item>
    <ContextMenu.Item onSelect={() => void vocabulary.place(name, { newGroupAt: group + 1 })}>
      New group below
    </ContextMenu.Item>
  {/if}
  {#each otherGroups as target (target)}
    <ContextMenu.Item onSelect={() => void vocabulary.place(name, { group: target })}>
      Move to #{target}
    </ContextMenu.Item>
  {/each}
{/if}
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
      {@const Icon = CATEGORY_ICON[option]}
      <Icon />
      {categoryLabel(option)}
    </ContextMenu.CheckboxItem>
  {/each}
</ContextMenu.Group>

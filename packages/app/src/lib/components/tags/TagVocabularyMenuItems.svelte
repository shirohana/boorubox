<script lang="ts">
  // The tag context menu's vocabulary items — a Danbooru look-up, "Edit
  // note…", pin/unpin, a pinned tag's own group moves, and the category
  // group — mounted wherever a tag's context menu opens (`tag-vocabulary`
  // design D9): the sidebar row, the inspector badge's menu, and the pinned
  // chip's own menu. A chip's tag is pinned by definition, so its menu shows
  // Unpin and the group moves without this component needing a flag to
  // suppress Pin.
  //
  // The group moves live here rather than on the chip itself
  // (`pinned-tag-groups` design D6) so a pinned tag offers the same moves
  // from every menu it has, sidebar and badge included, not only the chip.
  //
  // "Edit note…" lives here too (`tag-notes` design D10): a note is a
  // property of the tag, not of an image, so unlike "Rename artist…" once was
  // (`artist-entries` design D6, kept on the inspector's own tag chip menu
  // because the dialog's inputs came from an image) it belongs where every
  // tag menu shares it. `oneditnote` is required so a mount that forgets it
  // fails typecheck rather than offering a dead item — the dialog itself
  // cannot live here (this component's content unmounts on select), so each
  // host owns one, mounted unconditionally beside its menu. The two edit
  // items sit together, above the look-up (owner, 2026-09-28): both open a
  // dialog in the app, the look-up leaves for the browser.
  //
  // "Edit artist…" moved in here too (`artist-workflow` design D1, reversing
  // `artist-entries` D6): once an entry's own URLs, not an image's, are what
  // the dialog shows, an image is no longer required to open it, so the
  // sidebar row and the pinned chip can offer it exactly like the badge does.
  // `oneditartist` is required for the same reason `oneditnote` is; the item
  // itself renders only for a tag whose category is artist.
  //
  // Snippet-free, unlike `CollectionMenuItems`: every mount point here is a
  // `ContextMenu.Root` (never a dropdown), so this renders `ContextMenu.*`
  // primitives directly rather than taking them as snippets.
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link'
  import PencilIcon from '@lucide/svelte/icons/pencil'
  import PinIcon from '@lucide/svelte/icons/pin'
  import PinOffIcon from '@lucide/svelte/icons/pin-off'
  import Settings2Icon from '@lucide/svelte/icons/settings-2'
  import StickyNoteIcon from '@lucide/svelte/icons/sticky-note'
  import { vocabulary } from '$lib/api'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { CATEGORY_ORDER, categoryLabel } from '$lib/domain/tag-categories'
  import { danbooruLookup } from '$lib/domain/danbooru'
  import { CATEGORY_ICON } from './categories'
  import { openDanbooruLookup } from './danbooru-open'

  interface Props {
    name: string
    /** "Edit note…" was chosen — the host snapshots `name` and opens its own `TagNoteDialog`. */
    oneditnote: (name: string) => void
    /**
     * "Edit artist…" was chosen — the host builds its own `ArtistDialogRequest`
     * snapshot (`{ mode: 'edit', tag, adapter }`) and opens its own
     * `ArtistDialog` (`artist-workflow` design D2).
     */
    oneditartist: (tag: string) => void
    /**
     * "Manage pinned groups…" was chosen — the host opens its own
     * `PinnedGroupsDialog` (`pinned-group-management` design D9), the
     * `oneditnote` shape and required for the same reason.
     */
    onmanagegroups: () => void
  }

  let { name, oneditnote, oneditartist, onmanagegroups }: Props = $props()

  const group = $derived(vocabulary.groupOf(name))
  const pinned = $derived(group !== null)
  const category = $derived(vocabulary.categoryOf(name))
  const lookup = $derived(danbooruLookup(name, category))

  /**
   * Every group but the tag's own, in order — the "Move to <group>" items. A
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
  const alone = $derived(group !== null && vocabulary.pinnedGroups[group - 1]?.tags.length === 1)
</script>

{#if category === 'artist'}
  <!--
    First among the vocabulary items (`artist-workflow` design D1): the
    position the badge menu's artist item has held since `artist-entries`,
    so the owner's hand finds it where it always was.
  -->
  <ContextMenu.Item onSelect={() => oneditartist(name)}>
    <PencilIcon />
    Edit artist…
  </ContextMenu.Item>
{/if}
<ContextMenu.Item onSelect={() => oneditnote(name)}>
  <StickyNoteIcon />
  Edit note…
</ContextMenu.Item>
<ContextMenu.Item onSelect={() => openDanbooruLookup(lookup)}>
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
    A tag's own moves only insert or relocate it (`pinned-tag-groups`: no
    move up or move down for one tag; moving a whole group is
    `pinned-group-management`'s dialog): "New group above"/"below" is
    `NewGroupAt`, "Move to #x" is `Group(x)` (`pinned-tag-groups` design D3). Not for a tag alone in
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
      Move to {vocabulary.labelOf(target)}
    </ContextMenu.Item>
  {/each}
{/if}
{#if vocabulary.groupCount > 0}
  <ContextMenu.Item onSelect={onmanagegroups}>
    <Settings2Icon />
    Manage pinned groups…
  </ContextMenu.Item>
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

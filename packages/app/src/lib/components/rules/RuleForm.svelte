<script lang="ts">
  // Slot Settings · Rules, the editing half (`auto-tag-rules` design D11): the
  // legacy's four fields — name, pattern, "is a regular expression", tags — in
  // the frame's components. One form for both creating and editing: the
  // refusals are the same three either way (design D6), and a second form
  // would be the same four fields with a different button.
  import type { Rule } from '@boorubox/shared'
  import { errorText, rulesUpsert } from '$lib/api'
  import TagInput from '$lib/components/tags/TagInput.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Switch } from '$lib/components/ui/switch'
  import { tagList } from '$lib/domain/tag-utils'

  interface Props {
    /** The rule being edited, or `null` to create one. */
    rule: Rule | null
    /** The rule as Rust stored it — the caller re-reads the list from this. */
    onsaved: (rule: Rule) => void
    oncancel: () => void
  }

  let { rule, onsaved, oncancel }: Props = $props()

  let name = $state('')
  let pattern = $state('')
  let isRegex = $state(false)
  let tags = $state('')
  let saving = $state(false)
  let error = $state<string | null>(null)

  // The fields follow whichever rule the caller hands over, so clicking Edit on
  // a second row while the first is open loads that row rather than leaving the
  // form showing the one before it.
  $effect(() => {
    name = rule?.name ?? ''
    pattern = rule?.pattern ?? ''
    isRegex = rule?.isRegex ?? false
    tags = rule?.tags.join(' ') ?? ''
    error = null
  })

  /**
   * Rust refuses an empty name, no tags and an unusable regular expression with
   * the reason (design D6), and that refusal is what is shown: a copy of those
   * three checks here would be a second definition of what a valid rule is, and
   * the engine is the only authority on the third.
   */
  async function save() {
    if (saving) return
    saving = true
    error = null
    try {
      onsaved(await rulesUpsert({
        id: rule?.id,
        name,
        pattern,
        isRegex,
        tags: tagList(tags),
        // Editing must not switch a rule off, and a new rule is on: the enable
        // switch is the table's, so this form never changes it.
        enabled: rule?.enabled ?? true,
      }))
    } catch (cause) {
      // The fields keep what was typed: nothing was saved, and retyping it is
      // the last thing anyone wants after being told why.
      error = errorText(cause)
    } finally {
      saving = false
    }
  }
</script>

<form
  class="flex flex-col gap-3 rounded-lg border border-border p-3"
  onsubmit={(event) => {
    event.preventDefault()
    void save()
  }}
>
  <div class="grid gap-3 sm:grid-cols-2">
    <div class="grid gap-1.5">
      <Label for="rule-name" class="text-xs text-muted-foreground">Name</Label>
      <Input id="rule-name" class="h-8" bind:value={name} autocomplete="off" />
    </div>

    <div class="grid gap-1.5">
      <Label for="rule-pattern" class="text-xs text-muted-foreground">
        Pattern
        <span class="font-normal">— empty matches every image</span>
      </Label>
      <Input
        id="rule-pattern"
        class="h-8 font-mono"
        bind:value={pattern}
        autocomplete="off"
        spellcheck={false}
      />
    </div>
  </div>

  <div class="grid gap-1.5">
    <Label for="rule-tags" class="text-xs text-muted-foreground">
      Tags
      <span class="font-normal">— `rating:s` sets the rating instead of being stored</span>
    </Label>
    <TagInput
      id="rule-tags"
      bind:value={tags}
      label="Tags this rule adds"
      placeholder="Tags, separated by spaces"
      class="h-8"
    />
  </div>

  <div class="flex items-center gap-2">
    <Switch id="rule-is-regex" bind:checked={isRegex} />
    <Label for="rule-is-regex" class="text-xs">Pattern is a regular expression</Label>
  </div>

  {#if error}
    <p class="text-xs text-destructive">{error}</p>
  {/if}

  <div class="flex justify-end gap-2">
    <Button type="button" size="sm" variant="ghost" onclick={oncancel}>Cancel</Button>
    <Button type="submit" size="sm" disabled={saving}>
      {rule ? 'Save rule' : 'Add rule'}
    </Button>
  </div>
</form>

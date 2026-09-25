<script lang="ts">
  // Slot Settings · Rules (`auto-tag-rules` design D11): the whole section —
  // Import, Export and Run above, the form, then the table. Not a route and not
  // a nav item: rules are configuration of the library, which is what /settings
  // is, and filling both would put one screen behind two doors.
  import type { ExportProgress, Rule, RuleListEntry, RulesRunReport } from '@boorubox/shared'
  import PlusIcon from '@lucide/svelte/icons/plus'
  import {
    errorText,
    library,
    onRulesProgress,
    pickRulesExportPath,
    pickRulesImportPath,
    rulesExport,
    rulesImport,
    rulesList,
    rulesRun,
    vocabulary,
  } from '$lib/api'
  import { Button } from '$lib/components/ui/button'
  import { Progress } from '$lib/components/ui/progress'
  import RuleForm from './RuleForm.svelte'
  import RuleList from './RuleList.svelte'

  let entries = $state<RuleListEntry[]>([])
  /** Whether the form is open, and on which rule — `null` is a new one. */
  let formOpen = $state(false)
  let editing = $state<Rule | null>(null)
  let newIds = $state(new Set<string>())
  let running = $state(false)
  let progress = $state<ExportProgress | null>(null)
  let report = $state<RulesRunReport | null>(null)
  let imported = $state<string | null>(null)
  let error = $state<string | null>(null)

  // Re-read when the open library changes: the switch menu on this very screen
  // can swap the library underneath the list, and rules belong to the library.
  // The path, not the status: a capture replaces the whole status object, and
  // depending on it would re-read the whole list on every one of them.
  const libraryPath = $derived(library.status?.libraryPath ?? null)
  $effect(() => {
    void libraryPath
    void load()
  })

  // The run's ticks, for as long as the section is on screen. Subscribed once
  // rather than per run: `listen` resolves asynchronously, and a subscription
  // started when the run starts misses the first images.
  $effect(() => {
    const subscription = onRulesProgress((tick) => (progress = tick))
    return () => {
      void subscription.then((stop) => stop())
    }
  })

  async function load() {
    try {
      entries = await rulesList()
      error = null
    } catch (cause) {
      error = errorText(cause)
    }
  }

  async function guard(action: () => Promise<void>) {
    error = null
    try {
      await action()
    } catch (cause) {
      error = errorText(cause)
    }
  }

  const exportRules = () =>
    guard(async () => {
      const path = await pickRulesExportPath()
      if (path === null) return
      await rulesExport(path)
    })

  /**
   * The ids the file actually added, so the table can mark them (design D11).
   * The report counts, it does not name, and a count is not something anyone
   * can find in a list of thirty rules.
   */
  const importRules = () =>
    guard(async () => {
      const path = await pickRulesImportPath()
      if (path === null) return
      const before = new Set(entries.map((entry) => entry.rule.id))
      const result = await rulesImport(path)
      await load()
      newIds = new Set(
        entries.map((entry) => entry.rule.id).filter((id) => !before.has(id)),
      )
      imported = `Imported ${result.imported.toLocaleString()}, skipped ${result.skipped.toLocaleString()} already here.`
    })

  const run = () =>
    guard(async () => {
      running = true
      progress = null
      report = null
      try {
        report = await rulesRun()
        await load()
        // `tag-vocabulary` design D5: a rule's tags can create an artist over
        // every image it matches, and this run is the only place that
        // happens outside a single save.
        void vocabulary.refresh()
      } finally {
        running = false
        progress = null
      }
    })
</script>

<section class="flex flex-col gap-4">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h2 class="text-sm font-semibold">Rules</h2>
    <div class="flex flex-wrap gap-2">
      <Button size="sm" variant="outline" onclick={importRules}>Import…</Button>
      <Button size="sm" variant="outline" onclick={exportRules}>Export…</Button>
      <Button size="sm" variant="secondary" disabled={running} onclick={run}>
        {running ? 'Running…' : 'Run on existing images'}
      </Button>
    </div>
  </div>

  <p class="text-sm text-muted-foreground">
    A rule adds its tags to every image whose title, site or capture details match its pattern,
    as the image enters the library. Rules are stored with the library, so copying the folder
    carries them.
  </p>

  <!--
    A run over ten thousand images is minutes of nothing on screen otherwise
    (design D12); the ticks are the same `{ done, total }` an import emits.
  -->
  {#if running}
    <div class="flex flex-col gap-1">
      <Progress value={progress?.done ?? 0} max={progress?.total ?? 1} />
      <p class="text-xs text-muted-foreground tabular-nums">
        {(progress?.done ?? 0).toLocaleString()} of {(progress?.total ?? 0).toLocaleString()} images
      </p>
    </div>
  {/if}

  {#if imported}
    <p class="text-sm">{imported}</p>
  {/if}

  <!--
    Per rule, not per image: the question a run asks is "did the rule I just
    wrote do anything", and a list the size of the library answers nobody
    (design D9).
  -->
  {#if report}
    <div class="rounded-lg border border-border p-3 text-sm">
      <div class="flex items-baseline justify-between gap-2">
        <p>
          Examined {report.examined.toLocaleString()}
          {report.examined === 1 ? 'image' : 'images'}, changed
          {report.changed.toLocaleString()}.
        </p>
        <Button size="xs" variant="ghost" onclick={() => (report = null)}>Dismiss</Button>
      </div>

      {#if report.rules.length > 0}
        <dl class="mt-2 grid grid-cols-[minmax(0,1fr)_auto] gap-x-4 gap-y-1 text-xs">
          {#each report.rules as count (count.id)}
            <dt class="truncate text-muted-foreground">{count.name}</dt>
            <dd class="tabular-nums">{count.matched.toLocaleString()}</dd>
          {/each}
        </dl>
      {/if}

      {#if report.invalid.length > 0}
        <ul class="mt-2 text-xs text-destructive">
          {#each report.invalid as invalid (invalid.id)}
            <li>{invalid.name} — skipped: {invalid.patternError}</li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {#if formOpen}
    <RuleForm
      rule={editing}
      onsaved={() => {
        formOpen = false
        editing = null
        void load()
      }}
      oncancel={() => {
        formOpen = false
        editing = null
      }}
    />
  {:else}
    <div>
      <Button size="sm" onclick={() => (formOpen = true)}>
        <PlusIcon />
        New rule
      </Button>
    </div>
  {/if}

  <RuleList
    {entries}
    {newIds}
    onedit={(entry) => {
      editing = entry.rule
      formOpen = true
    }}
    onchanged={load}
    onerror={(message) => (error = message)}
  />

  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}
</section>

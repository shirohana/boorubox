<script lang="ts">
  // The draggable top edge of a sidebar section (`sidebar-inspector-polish`
  // design D8): a section's bottom is pinned against the sections below it,
  // so a handle there cannot be dragged past them (`collections`, `notes`
  // specs, owner 2026-09-24). The handle is on the top edge instead, and
  // dragging it up therefore grows the section below it. `clampHeight`
  // (`./section-resizer.ts`) does the math; this is the DOM half — pointer
  // capture so the drag tracks past the strip itself, and `min` / `max` are
  // captured at pointer-down so a resize during the drag never rereads them
  // mid-gesture. The drag ends in `onlostpointercapture`, which fires on a
  // pointerup, a pointercancel and a window blur alike, so a drag that ends
  // off-screen still stops.
  import { clampHeight } from './section-resizer'

  interface Props {
    /** The section's current height in px. */
    height: number
    min: number
    /**
     * A plain ceiling, or a function called once at pointer-down. A function
     * is how a section whose room depends on the *other* section's current
     * height (the tag list's floor minus what it's already given up, design
     * D8's ceiling amendment) reads that room fresh on each drag rather than
     * at mount, without rereading it mid-gesture.
     */
    max: number | (() => number)
    onresize: (next: number) => void
    label: string
  }

  let { height, min, max, onresize, label }: Props = $props()

  let dragging = $state(false)
  let startHeight = 0
  let startY = 0
  let startMin = 0
  let startMax = 0
</script>

<div
  role="separator"
  aria-orientation="horizontal"
  aria-label={label}
  class="
    h-1.5 shrink-0 cursor-row-resize touch-none rounded-full
    hover:bg-sidebar-accent
    {dragging ? 'bg-sidebar-accent' : ''}
  "
  onpointerdown={(event) => {
    if (event.button !== 0) return
    event.preventDefault()
    startHeight = height
    startY = event.clientY
    startMin = min
    startMax = typeof max === 'function' ? max() : max
    dragging = true
    event.currentTarget.setPointerCapture(event.pointerId)
  }}
  onpointermove={(event) => {
    if (!dragging) return
    onresize(clampHeight(startHeight, startY - event.clientY, startMin, startMax))
  }}
  onlostpointercapture={() => {
    dragging = false
  }}
></div>

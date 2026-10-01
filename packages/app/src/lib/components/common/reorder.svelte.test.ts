// @vitest-environment jsdom

import { flushSync, mount, unmount } from 'svelte'
import { afterAll, afterEach, expect, it, vi } from 'vitest'
import * as pointerDrag from './pointer-drag'
import { reorderable } from './reorder'
import ReorderHandle from './ReorderHandle.svelte'

function pointer(type: string, clientY = 0) {
  const event = new Event(type, { bubbles: true, cancelable: true })
  Object.assign(event, { clientX: 0, clientY, pointerId: 1, button: 0 })
  return event
}

// jsdom has no layout: the row under the pointer is whichever one the test
// says `elementFromPoint` answers.
let under: Element | null = null
const originalElementFromPoint = document.elementFromPoint
document.elementFromPoint = () => under

function setup() {
  const list = document.createElement('ul')
  document.body.appendChild(list)
  const handles: ReturnType<typeof mount>[] = []
  for (const index of [0, 1, 2]) {
    const row = document.createElement('li')
    row.dataset.reorderIndex = String(index)
    row.getBoundingClientRect = () => ({ top: 0, height: 20 }) as DOMRect
    list.appendChild(row)
    handles.push(mount(ReorderHandle, { target: row }))
  }
  flushSync()
  const onmove = vi.fn()
  const action = reorderable(list, { onmove })
  const rows = [...list.querySelectorAll('li')]
  const grips = rows.map((row) => {
    const grip = row.querySelector('button')!
    grip.setPointerCapture = vi.fn()
    grip.releasePointerCapture = vi.fn()
    return grip
  })
  const teardown = () => {
    action.destroy()
    handles.forEach((handle) => unmount(handle))
    list.remove()
  }
  return { rows, grips, onmove, teardown }
}

afterEach(() => {
  under = null
})

afterAll(() => {
  document.elementFromPoint = originalElementFromPoint
})

it('a drag from a handle dropped on the lower half of another row reports from and to', () => {
  const { rows, grips, onmove, teardown } = setup()

  grips[0].dispatchEvent(pointer('pointerdown'))
  under = rows[2]
  grips[0].dispatchEvent(pointer('pointermove', 15))
  expect(rows[2].getAttribute('data-reorder-drop')).toBe('after')
  grips[0].dispatchEvent(pointer('pointerup', 15))

  expect(onmove).toHaveBeenCalledWith(0, 2)
  expect(rows[2].hasAttribute('data-reorder-drop')).toBe(false)
  teardown()
})

it('the upper half of a row above the dragged one lands before it', () => {
  const { rows, grips, onmove, teardown } = setup()

  grips[2].dispatchEvent(pointer('pointerdown', 50))
  under = rows[0]
  grips[2].dispatchEvent(pointer('pointermove', 5))
  expect(rows[0].getAttribute('data-reorder-drop')).toBe('before')
  grips[2].dispatchEvent(pointer('pointerup', 5))

  expect(onmove).toHaveBeenCalledWith(2, 0)
  teardown()
})

it('a drop that leaves the item where it was reports nothing', () => {
  const { rows, grips, onmove, teardown } = setup()

  grips[1].dispatchEvent(pointer('pointerdown', 50))
  under = rows[1]
  grips[1].dispatchEvent(pointer('pointermove', 5))
  grips[1].dispatchEvent(pointer('pointerup', 5))

  expect(onmove).not.toHaveBeenCalled()
  teardown()
})

it('the drop marker follows the pointer and clears when the drag is cancelled', () => {
  const { rows, grips, onmove, teardown } = setup()

  grips[0].dispatchEvent(pointer('pointerdown'))
  under = rows[2]
  grips[0].dispatchEvent(pointer('pointermove', 15))
  under = rows[1]
  grips[0].dispatchEvent(pointer('pointermove', 5))
  expect(rows[2].hasAttribute('data-reorder-drop')).toBe(false)
  expect(rows[1].getAttribute('data-reorder-drop')).toBe('before')

  grips[0].dispatchEvent(pointer('pointercancel', 5))
  expect(rows[1].hasAttribute('data-reorder-drop')).toBe(false)
  expect(onmove).not.toHaveBeenCalled()
  teardown()
})

it('a drop outside every row reports nothing', () => {
  const { grips, onmove, teardown } = setup()

  grips[0].dispatchEvent(pointer('pointerdown'))
  under = document.body
  grips[0].dispatchEvent(pointer('pointermove', 15))
  grips[0].dispatchEvent(pointer('pointerup', 15))

  expect(onmove).not.toHaveBeenCalled()
  teardown()
})

it('a press that does not start at a handle reports nothing', () => {
  const { rows, onmove, teardown } = setup()

  rows[0].dispatchEvent(pointer('pointerdown'))
  under = rows[2]
  rows[0].dispatchEvent(pointer('pointermove', 15))
  rows[0].dispatchEvent(pointer('pointerup', 15))

  expect(onmove).not.toHaveBeenCalled()
  teardown()
})

it('reports the pointer to the edge scroller on every move and stops it on drop', () => {
  const at = vi.fn()
  const stop = vi.fn()
  vi.spyOn(pointerDrag, 'edgeScroller').mockReturnValue({ at, stop })
  const { rows, grips, teardown } = setup()

  grips[0].dispatchEvent(pointer('pointerdown'))
  under = rows[2]
  grips[0].dispatchEvent(pointer('pointermove', 15))
  expect(at).toHaveBeenCalledWith(0, 15)
  grips[0].dispatchEvent(pointer('pointerup', 15))
  expect(stop).toHaveBeenCalled()

  teardown()
  vi.restoreAllMocks()
})

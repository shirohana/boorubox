// @vitest-environment jsdom

import { flushSync, mount, unmount } from 'svelte'
import { expect, it, vi } from 'vitest'
import { reorderable } from './reorder'
import ReorderHandle from './ReorderHandle.svelte'

function dragEvent(type: string, clientY = 0) {
  const event = new Event(type, { bubbles: true, cancelable: true })
  Object.defineProperty(event, 'clientY', { value: clientY })
  Object.defineProperty(event, 'dataTransfer', {
    value: { setData: vi.fn(), setDragImage: vi.fn(), effectAllowed: '', dropEffect: '' },
  })
  return event
}

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
  const teardown = () => {
    action.destroy()
    handles.forEach((handle) => unmount(handle))
    list.remove()
  }
  return { rows, onmove, teardown }
}

it('a drag from a handle dropped on the lower half of another row reports from and to', () => {
  const { rows, onmove, teardown } = setup()

  rows[0].querySelector('button')!.dispatchEvent(dragEvent('dragstart'))
  const over = dragEvent('dragover', 15)
  rows[2].dispatchEvent(over)
  expect(over.defaultPrevented).toBe(true)
  expect(rows[2].getAttribute('data-reorder-drop')).toBe('after')
  rows[2].dispatchEvent(dragEvent('drop', 15))

  expect(onmove).toHaveBeenCalledWith(0, 2)
  expect(rows[2].hasAttribute('data-reorder-drop')).toBe(false)
  teardown()
})

it('the upper half of a row above the dragged one lands before it', () => {
  const { rows, onmove, teardown } = setup()

  rows[2].querySelector('button')!.dispatchEvent(dragEvent('dragstart'))
  rows[0].dispatchEvent(dragEvent('dragover', 5))
  rows[0].dispatchEvent(dragEvent('drop', 5))

  expect(onmove).toHaveBeenCalledWith(2, 0)
  teardown()
})

it('a drop that leaves the item where it was reports nothing', () => {
  const { rows, onmove, teardown } = setup()

  rows[1].querySelector('button')!.dispatchEvent(dragEvent('dragstart'))
  rows[1].dispatchEvent(dragEvent('dragover', 5))
  rows[1].dispatchEvent(dragEvent('drop', 5))

  expect(onmove).not.toHaveBeenCalled()
  teardown()
})

it('the drop marker clears when the pointer leaves the list', () => {
  const { rows, teardown } = setup()

  rows[0].querySelector('button')!.dispatchEvent(dragEvent('dragstart'))
  rows[2].dispatchEvent(dragEvent('dragover', 15))
  expect(rows[2].hasAttribute('data-reorder-drop')).toBe(true)

  const stay = dragEvent('dragleave')
  Object.defineProperty(stay, 'relatedTarget', { value: rows[1] })
  rows[2].dispatchEvent(stay)
  expect(rows[2].hasAttribute('data-reorder-drop')).toBe(true)

  const leave = dragEvent('dragleave')
  Object.defineProperty(leave, 'relatedTarget', { value: document.body })
  rows[2].dispatchEvent(leave)
  expect(rows[2].hasAttribute('data-reorder-drop')).toBe(false)
  teardown()
})

it('a drag that does not start at a handle reports nothing', () => {
  const { rows, onmove, teardown } = setup()

  rows[0].dispatchEvent(dragEvent('dragstart'))
  rows[2].dispatchEvent(dragEvent('dragover', 15))
  rows[2].dispatchEvent(dragEvent('drop', 15))

  expect(onmove).not.toHaveBeenCalled()
  teardown()
})

// @vitest-environment jsdom

import { afterEach, expect, it, vi } from 'vitest'
import { pointerDrag } from './pointer-drag'

function pointer(type: string, x = 0, y = 0, init: PointerEventInit = {}) {
  const event = new Event(type, { bubbles: true, cancelable: true })
  Object.assign(event, { clientX: x, clientY: y, pointerId: 1, button: 0, ...init })
  return event as PointerEvent
}

function setup() {
  const handle = document.createElement('button')
  document.body.appendChild(handle)
  handle.setPointerCapture = vi.fn()
  handle.releasePointerCapture = vi.fn()
  const handlers = { onstart: vi.fn(), onmove: vi.fn(), onend: vi.fn() }
  const action = pointerDrag(handle, handlers)
  return { handle, handlers, action }
}

afterEach(() => {
  document.body.innerHTML = ''
})

it('a move under the threshold never starts', () => {
  const { handle, handlers } = setup()
  handle.dispatchEvent(pointer('pointerdown', 10, 10))
  handle.dispatchEvent(pointer('pointermove', 12, 12))
  handle.dispatchEvent(pointer('pointerup', 12, 12))
  expect(handlers.onstart).not.toHaveBeenCalled()
  expect(handlers.onmove).not.toHaveBeenCalled()
  expect(handlers.onend).not.toHaveBeenCalled()
})

it('a move past the threshold starts and reports coordinates', () => {
  const { handle, handlers } = setup()
  handle.dispatchEvent(pointer('pointerdown', 10, 10))
  handle.dispatchEvent(pointer('pointermove', 10, 20))
  handle.dispatchEvent(pointer('pointermove', 30, 40))
  expect(handlers.onstart).toHaveBeenCalledOnce()
  expect(handlers.onstart).toHaveBeenCalledWith(10, 20, handle)
  expect(handlers.onmove).toHaveBeenNthCalledWith(1, 10, 20)
  expect(handlers.onmove).toHaveBeenNthCalledWith(2, 30, 40)
})

it('pointerup ends with dropped true and releases the capture', () => {
  const { handle, handlers } = setup()
  handle.dispatchEvent(pointer('pointerdown', 0, 0))
  expect(handle.setPointerCapture).toHaveBeenCalledWith(1)
  handle.dispatchEvent(pointer('pointermove', 0, 20))
  handle.dispatchEvent(pointer('pointerup', 5, 25))
  expect(handlers.onend).toHaveBeenCalledExactlyOnceWith(5, 25, true)
  expect(handle.releasePointerCapture).toHaveBeenCalledWith(1)
})

it('pointercancel ends with dropped false', () => {
  const { handle, handlers } = setup()
  handle.dispatchEvent(pointer('pointerdown'))
  handle.dispatchEvent(pointer('pointermove', 0, 20))
  handle.dispatchEvent(pointer('pointercancel', 0, 20))
  expect(handlers.onend).toHaveBeenCalledExactlyOnceWith(0, 20, false)
})

it('Escape ends with dropped false and a later pointerup adds nothing', () => {
  const { handle, handlers } = setup()
  handle.dispatchEvent(pointer('pointerdown'))
  handle.dispatchEvent(pointer('pointermove', 0, 20))
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
  handle.dispatchEvent(pointer('pointerup', 0, 20))
  expect(handlers.onend).toHaveBeenCalledExactlyOnceWith(0, 20, false)
})

it('Escape during a started drag does not reach the document, and before the start it does', () => {
  const { handle } = setup()
  const reached = vi.fn()
  document.addEventListener('keydown', reached)
  handle.dispatchEvent(pointer('pointerdown'))
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
  expect(reached).toHaveBeenCalledTimes(1)

  handle.dispatchEvent(pointer('pointerdown'))
  handle.dispatchEvent(pointer('pointermove', 0, 20))
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
  expect(reached).toHaveBeenCalledTimes(1)
  document.removeEventListener('keydown', reached)
})

it('a capture lost mid-drag ends with dropped false', () => {
  const { handle, handlers } = setup()
  handle.dispatchEvent(pointer('pointerdown'))
  handle.dispatchEvent(pointer('pointermove', 0, 20))
  handle.dispatchEvent(pointer('lostpointercapture', 0, 20))
  expect(handlers.onend).toHaveBeenCalledExactlyOnceWith(0, 20, false)
})

it('a secondary button does not arm', () => {
  const { handle, handlers } = setup()
  handle.dispatchEvent(pointer('pointerdown', 0, 0, { button: 2 }))
  handle.dispatchEvent(pointer('pointermove', 0, 20))
  expect(handle.setPointerCapture).not.toHaveBeenCalled()
  expect(handlers.onstart).not.toHaveBeenCalled()
})

it('with a selector only a press inside a match arms', () => {
  const root = document.createElement('div')
  root.innerHTML = '<span data-grip><i></i></span><p></p>'
  document.body.appendChild(root)
  const grip = root.querySelector<HTMLElement>('[data-grip]')!
  grip.setPointerCapture = vi.fn()
  grip.releasePointerCapture = vi.fn()
  const onstart = vi.fn()
  pointerDrag(root, { onstart }, { selector: '[data-grip]' })

  root.querySelector('p')!.dispatchEvent(pointer('pointerdown'))
  root.querySelector('p')!.dispatchEvent(pointer('pointermove', 0, 20))
  expect(onstart).not.toHaveBeenCalled()

  root.querySelector('i')!.dispatchEvent(pointer('pointerdown'))
  expect(grip.setPointerCapture).toHaveBeenCalledWith(1)
  grip.dispatchEvent(pointer('pointermove', 0, 20))
  expect(onstart).toHaveBeenCalledWith(0, 20, grip)
})

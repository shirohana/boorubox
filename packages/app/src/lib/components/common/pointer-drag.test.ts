// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from 'vitest'
import { edgeScroller, pointerDrag } from './pointer-drag'

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

/** A box that overflows by 800px, with a plain-number `scrollTop` and a rect of 100..300. */
function scrollBox(parent: HTMLElement, overflowY = 'auto') {
  const box = document.createElement('div')
  box.style.overflowY = overflowY
  box.getBoundingClientRect = () => ({ top: 100, bottom: 300 }) as DOMRect
  Object.defineProperty(box, 'scrollHeight', { value: 1000, configurable: true })
  Object.defineProperty(box, 'clientHeight', { value: 200, configurable: true })
  let top = 0
  Object.defineProperty(box, 'scrollTop', { get: () => top, set: (v: number) => (top = v), configurable: true })
  parent.appendChild(box)
  return box
}

function stubFrames() {
  const frames: FrameRequestCallback[] = []
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => frames.push(callback))
  vi.stubGlobal('cancelAnimationFrame', () => frames.splice(0))
  return frames
}

function pointerOver(element: Element | null) {
  document.elementFromPoint = () => element
}

describe('edgeScroller', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('scrolls the nearest scrollable ancestor of the element under the pointer', () => {
    const frames = stubFrames()
    const outer = scrollBox(document.body)
    const inner = scrollBox(outer)
    const leaf = document.createElement('span')
    inner.appendChild(leaf)
    pointerOver(leaf)

    edgeScroller().at(0, 290)
    frames.shift()!(0)

    expect(inner.scrollTop).toBeGreaterThan(2)
    expect(outer.scrollTop).toBe(0)
  })

  it('skips an ancestor that does not overflow or does not scroll', () => {
    const frames = stubFrames()
    const outer = scrollBox(document.body)
    const hidden = scrollBox(outer, 'hidden')
    pointerOver(hidden)

    edgeScroller().at(0, 290)
    frames.shift()!(0)

    expect(hidden.scrollTop).toBe(0)
    expect(outer.scrollTop).toBeGreaterThan(2)
  })

  it('falls back to the document scrolling element, measured against the viewport', () => {
    const frames = stubFrames()
    const root = document.documentElement
    Object.defineProperty(document, 'scrollingElement', { value: root, configurable: true })
    Object.defineProperty(root, 'scrollTop', { value: 0, writable: true, configurable: true })
    pointerOver(document.body)

    edgeScroller().at(0, window.innerHeight - 5)
    frames.shift()!(0)

    expect(root.scrollTop).toBeGreaterThan(2)
  })

  it('steps on the next frame in the bottom band, up in the top band, and not outside them', () => {
    const frames = stubFrames()
    const box = scrollBox(document.body)
    box.scrollTop = 500
    pointerOver(box)
    const scroller = edgeScroller()

    scroller.at(0, 200)
    expect(frames).toHaveLength(0)

    scroller.at(0, 110)
    frames.shift()!(0)
    expect(box.scrollTop).toBeLessThan(500)

    box.scrollTop = 500
    scroller.at(0, 290)
    expect(box.scrollTop).toBe(500)
    frames.shift()!(0)
    expect(box.scrollTop).toBeGreaterThan(500)
    expect(frames).toHaveLength(1)
  })

  it('stop ends the loop', () => {
    const frames = stubFrames()
    const box = scrollBox(document.body)
    pointerOver(box)
    const scroller = edgeScroller()

    scroller.at(0, 290)
    scroller.stop()

    expect(frames).toHaveLength(0)
  })

  it('stops stepping when scrollTop no longer changes, and reports each step to onscroll', () => {
    const frames = stubFrames()
    const box = scrollBox(document.body)
    let top = 0
    Object.defineProperty(box, 'scrollTop', { get: () => top, set: (v: number) => (top = Math.min(v, 5)), configurable: true })
    pointerOver(box)
    const onscroll = vi.fn()

    edgeScroller(onscroll).at(0, 290)
    frames.shift()!(0)
    expect(frames).toHaveLength(1)
    frames.shift()!(0)

    expect(frames).toHaveLength(0)
    expect(onscroll).toHaveBeenCalledTimes(2)
  })
})

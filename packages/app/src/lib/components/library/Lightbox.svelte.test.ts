// @vitest-environment jsdom

import type { ImageRecord } from '@boorubox/shared'
import type { SearchResults } from '$lib/api'
import { clearMocks, mockConvertFileSrc, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, unmount } from 'svelte'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { img } from '$lib/domain/image-fixture'
import Lightbox from './Lightbox.svelte'

const platform = vi.hoisted(() => ({ isWindows: false }))
vi.mock('$lib/platform', () => ({
  get isWindows() {
    return platform.isWindows
  },
}))

beforeEach(() => {
  platform.isWindows = false
  // jsdom has no layout and no `showModal`; the viewport must measure non-zero
  // for a click to reach the zoom, so the contrast with a video is real.
  HTMLDialogElement.prototype.showModal ??= function showModal() {}
  HTMLDialogElement.prototype.close ??= function close() {}
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(800)
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(600)
  vi.stubGlobal('ResizeObserver', class {
    callback: (entries: { target: Element }[]) => void
    constructor(callback: (entries: { target: Element }[]) => void) {
      this.callback = callback
    }

    observe(target: Element) {
      this.callback([{ target }])
    }

    unobserve() {}
    disconnect() {}
  })
  mockIPC(() => null)
  mockConvertFileSrc('macos')
})

afterEach(() => {
  clearMocks()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  document.body.innerHTML = ''
})

const video = img({ id: 'v', ext: 'mp4', mime: 'video/mp4', durationMs: 8033 })

interface Callbacks { onmove?: (index: number) => void, onclose?: () => void }

function open(record: ImageRecord, callbacks: Callbacks = {}) {
  const results = {
    total: 2,
    groups: [],
    at: () => record,
    ensureRange: () => {},
  } as unknown as SearchResults
  const target = document.createElement('div')
  document.body.appendChild(target)
  const instance = mount(Lightbox, {
    target,
    props: {
      results,
      index: 0,
      libraryPath: '/lib',
      columns: 4,
      mode: 'gallery' as const,
      actions: { trash: vi.fn(), restore: vi.fn(), deleteForever: vi.fn() },
      clickZoomCeiling: 4,
      tagQuery: '',
      onquery: () => {},
      onartistsaved: () => {},
      onmove: callbacks.onmove ?? (() => {}),
      onclose: callbacks.onclose ?? (() => {}),
    },
  })
  flushSync()
  return { target, instance }
}

function measure(el: HTMLElement, width: number, height: number, key: 'video' | 'image') {
  if (key === 'video') {
    Object.defineProperty(el, 'videoWidth', { value: width })
    Object.defineProperty(el, 'videoHeight', { value: height })
    el.dispatchEvent(new Event('loadedmetadata'))
  } else {
    Object.defineProperty(el, 'naturalWidth', { value: width })
    Object.defineProperty(el, 'naturalHeight', { value: height })
    el.dispatchEvent(new Event('load'))
  }
  flushSync()
}

it('renders a video record as a looping, muted, autoplaying <video> with controls', () => {
  const { target, instance } = open(video)
  const el = target.querySelector('video')!
  expect(el).not.toBeNull()
  expect(target.querySelector('img')).toBeNull()
  expect(el.hasAttribute('loop')).toBe(true)
  expect(el.hasAttribute('autoplay')).toBe(true)
  expect(el.hasAttribute('controls')).toBe(true)
  expect(el.hasAttribute('playsinline')).toBe(true)
  expect(el.muted).toBe(true)
  expect(el.getAttribute('tabindex')).toBe('-1')
  expect(el.getAttribute('src')).toContain('v.mp4')
  unmount(instance)
})

it('leaves a video\'s size alone under the wheel', () => {
  const { target, instance } = open(video)
  const el = target.querySelector('video')!
  measure(el, 1664, 1024, 'video')
  const style = el.getAttribute('style')
  expect(style).toContain('width:')

  el.dispatchEvent(new WheelEvent('wheel', { deltaY: -200, ctrlKey: true, bubbles: true, cancelable: true }))
  flushSync()

  expect(el.getAttribute('style')).toBe(style)
  unmount(instance)
})

function pressOnFocusedVideo(el: HTMLVideoElement, key: string) {
  el.focus()
  el.dispatchEvent(new Event('pointerup', { bubbles: true }))
  flushSync()
  ;(document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }))
  flushSync()
}

it('closes on Space after a click on the video', () => {
  vi.spyOn(HTMLDialogElement.prototype, 'close').mockImplementation(function (this: HTMLDialogElement) {
    this.dispatchEvent(new Event('close'))
  })
  const onclose = vi.fn()
  const { target, instance } = open(video, { onclose })
  pressOnFocusedVideo(target.querySelector('video')!, ' ')
  expect(onclose).toHaveBeenCalled()
  unmount(instance)
})

it('moves on ArrowRight after a click on the video', () => {
  const onmove = vi.fn()
  const { target, instance } = open(video, { onmove })
  pressOnFocusedVideo(target.querySelector('video')!, 'ArrowRight')
  expect(onmove).toHaveBeenCalledWith(1)
  unmount(instance)
})

it('still zooms an image on click, and renders an <img>', () => {
  const frame = vi.spyOn(globalThis, 'requestAnimationFrame')
  const { target, instance } = open(img({ id: 'p' }))
  expect(target.querySelector('video')).toBeNull()
  const el = target.querySelector('img')!
  measure(el, 3000, 2000, 'image')

  el.click()

  expect(frame).toHaveBeenCalled()
  unmount(instance)
})

it('shows a message in the picture\'s place when the video cannot be played', () => {
  const { target, instance } = open(video)
  target.querySelector('video')!.dispatchEvent(new Event('error'))
  flushSync()
  expect(target.querySelector('video')).toBeNull()
  expect(target.textContent).toContain('This video cannot be played on this machine')
  unmount(instance)
})

it('still moves on the arrows after the video cannot be played', () => {
  const onmove = vi.fn()
  const { target, instance } = open(video, { onmove })
  target.querySelector('video')!.dispatchEvent(new Event('error'))
  flushSync()
  target.querySelector('dialog')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }))
  expect(onmove).toHaveBeenCalledWith(1)
  unmount(instance)
})

const hevc = img({ id: 'h', ext: 'mp4', mime: 'video/mp4', codec: 'hvc1', durationMs: 8033 })

function engineAnswers(answer: string) {
  return vi.spyOn(HTMLMediaElement.prototype, 'canPlayType').mockReturnValue(answer as CanPlayTypeResult)
}

it('names the codec and mounts no <video> when the engine will not decode it', () => {
  const probe = engineAnswers('')
  const { target, instance } = open(hevc)
  expect(target.querySelector('video')).toBeNull()
  expect(target.textContent).toContain('This machine\'s browser engine cannot decode HEVC (H.265).')
  expect(target.textContent).not.toContain('Microsoft')
  expect(probe).toHaveBeenCalledWith('video/mp4; codecs="hvc1.1.6.L93.B0"')
  unmount(instance)
})

it('adds the Windows sentence for HEVC on Windows', () => {
  engineAnswers('')
  platform.isWindows = true
  const { target, instance } = open(hevc)
  expect(target.textContent).toContain('Windows\' engine plays HEVC only with Microsoft\'s HEVC Video Extensions.')
  unmount(instance)
})

it('leaves the Windows sentence off a refused codec other than HEVC', () => {
  engineAnswers('')
  platform.isWindows = true
  const { target, instance } = open(img({ id: 'x', ext: 'webm', mime: 'video/webm', codec: 'vp09' }))
  expect(target.textContent).toContain('cannot decode VP9.')
  expect(target.textContent).not.toContain('Microsoft')
  unmount(instance)
})

it('still moves on the arrows over the refusal message', () => {
  engineAnswers('')
  const onmove = vi.fn()
  const { target, instance } = open(hevc, { onmove })
  target.querySelector('dialog')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }))
  expect(onmove).toHaveBeenCalledWith(1)
  unmount(instance)
})

it('mounts the <video> when the engine says probably', () => {
  engineAnswers('probably')
  const { target, instance } = open(hevc)
  expect(target.querySelector('video')).not.toBeNull()
  unmount(instance)
})

it('mounts the <video> without asking when no codec is recorded', () => {
  const probe = engineAnswers('')
  const { target, instance } = open(video)
  expect(target.querySelector('video')).not.toBeNull()
  expect(probe).not.toHaveBeenCalled()
  unmount(instance)
})

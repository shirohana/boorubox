// @vitest-environment jsdom

import type { ImageRecord } from '@boorubox/shared'
import type { SearchResults } from '$lib/api'
import { clearMocks, mockConvertFileSrc, mockIPC } from '@tauri-apps/api/mocks'
import { flushSync, mount, tick, unmount } from 'svelte'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { img } from '$lib/domain/image-fixture'
import Lightbox from './Lightbox.svelte'

const platform = vi.hoisted(() => ({ isWindows: false }))
vi.mock('$lib/platform', () => ({
  get isWindows() {
    return platform.isWindows
  },
}))

/** The `playback-sample-progress` handlers the viewer registered, so a test can push a payload. */
const events = vi.hoisted(() => ({ handlers: [] as ((event: { payload: unknown }) => void)[] }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: (_name: string, handler: (event: { payload: unknown }) => void) => {
    events.handlers.push(handler)
    return Promise.resolve(() => {})
  },
}))

/** Every command the mocked IPC saw; `playback_sample` stays pending until a test settles it. */
let calls: { cmd: string, payload: unknown }[] = []
let settleSample: { resolve: (ref: unknown) => void, reject: (reason: string) => void }

beforeEach(() => {
  platform.isWindows = false
  events.handlers = []
  calls = []
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
  mockIPC((cmd, payload) => {
    calls.push({ cmd, payload })
    if (cmd !== 'playback_sample') return null
    return new Promise((resolve, reject) => (settleSample = { resolve, reject }))
  })
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
  expect(target.textContent).toContain('this machine\'s browser engine cannot decode HEVC (H.265).')
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

const sampleCalls = () => calls.filter((call) => call.cmd === 'playback_sample')
const bar = (target: HTMLElement) => target.querySelector('[role="progressbar"]')

it('asks for the sample once, says it is converting and shows a bar', () => {
  engineAnswers('')
  const { target, instance } = open(hevc)
  expect(sampleCalls()).toEqual([{ cmd: 'playback_sample', payload: { id: 'h' } }])
  expect(target.textContent).toContain('Converting this video for playback: this machine\'s browser engine cannot decode HEVC (H.265).')
  expect(target.textContent).toContain('Please wait')
  expect(bar(target)).not.toBeNull()
  expect(target.querySelector('video')).toBeNull()
  unmount(instance)
})

it('moves the bar on a progress event for the record, not for another', () => {
  engineAnswers('')
  const { target, instance } = open(hevc)
  events.handlers.forEach((handler) => handler({ payload: { id: 'other', ratio: 0.9 } }))
  flushSync()
  expect(bar(target)?.getAttribute('aria-valuenow')).toBe('0')
  events.handlers.forEach((handler) => handler({ payload: { id: 'h', ratio: 0.5 } }))
  flushSync()
  expect(bar(target)?.getAttribute('aria-valuenow')).toBe('0.5')
  unmount(instance)
})

it('mounts the <video> on the sample once the command resolves', async () => {
  engineAnswers('')
  const { target, instance } = open(hevc)
  settleSample.resolve({ path: '/lib/.samples/ab/h.mp4', version: 7 })
  await tick()
  await tick()
  const el = target.querySelector('video')!
  expect(el.getAttribute('src')).toContain('.samples%2Fab%2Fh.mp4')
  expect(el.getAttribute('src')).toContain('?v=7')
  expect(el.hasAttribute('autoplay')).toBe(true)
  expect(bar(target)).toBeNull()
  unmount(instance)
})

it('shows the refusal and the reason when the command rejects', async () => {
  engineAnswers('')
  const { target, instance } = open(hevc)
  settleSample.reject('encoder exited with status 1')
  await tick()
  await tick()
  expect(target.querySelector('video')).toBeNull()
  expect(target.textContent).toContain('This machine\'s browser engine cannot decode HEVC (H.265).')
  expect(target.textContent).toContain('encoder exited with status 1')
  unmount(instance)
})

it('adds the Windows sentence to the converting message', () => {
  engineAnswers('')
  platform.isWindows = true
  const { target, instance } = open(hevc)
  expect(target.textContent).toContain('Microsoft\'s HEVC Video Extensions.')
  unmount(instance)
})

it('moves on while converting and does not ask again when it comes back', async () => {
  engineAnswers('')
  const records = [hevc, video]
  const view = $state({ index: 0 })
  const onmove = vi.fn()
  const target = document.createElement('div')
  document.body.appendChild(target)
  const instance = mount(Lightbox, {
    target,
    props: {
      results: { total: 2, groups: [], at: (i: number) => records[i], ensureRange: () => {} } as unknown as SearchResults,
      get index() { return view.index },
      libraryPath: '/lib',
      columns: 4,
      mode: 'gallery' as const,
      actions: { trash: vi.fn(), restore: vi.fn(), deleteForever: vi.fn() },
      clickZoomCeiling: 4,
      tagQuery: '',
      onquery: () => {},
      onartistsaved: () => {},
      onmove,
      onclose: () => {},
    },
  })
  flushSync()
  target.querySelector('dialog')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }))
  expect(onmove).toHaveBeenCalledWith(1)
  view.index = 1
  flushSync()
  view.index = 0
  flushSync()
  expect(sampleCalls()).toHaveLength(1)
  expect(target.textContent).toContain('Converting')
  unmount(instance)
})

it('never asks for a sample when the engine decodes the codec', () => {
  engineAnswers('probably')
  const { target, instance } = open(hevc)
  expect(sampleCalls()).toEqual([])
  expect(target.querySelector('video')?.getAttribute('src')).not.toContain('.samples')
  unmount(instance)
})

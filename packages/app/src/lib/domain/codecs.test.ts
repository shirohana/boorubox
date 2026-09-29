import { expect, it } from 'vitest'
import { canDecode, codecName, codecsString } from './codecs'

it('spells a codecs parameter for each known code and passes an unknown one through', () => {
  expect(codecsString('video/mp4', 'hvc1')).toBe('video/mp4; codecs="hvc1.1.6.L93.B0"')
  expect(codecsString('video/mp4', 'avc1')).toBe('video/mp4; codecs="avc1.42E01E"')
  expect(codecsString('video/webm', 'vp08')).toBe('video/webm; codecs="vp8"')
  expect(codecsString('video/webm', 'vp09')).toBe('video/webm; codecs="vp09.00.10.08"')
  expect(codecsString('video/mp4', 'zzzz')).toBe('video/mp4; codecs="zzzz"')
})

it('names the codecs for a reader, and an unknown code as itself', () => {
  expect(codecName('hvc1')).toBe('HEVC (H.265)')
  expect(codecName('avc1')).toBe('H.264')
  expect(codecName('vp08')).toBe('VP8')
  expect(codecName('vp09')).toBe('VP9')
  expect(codecName('zzzz')).toBe('zzzz')
})

it('decodes when nothing is recorded, without asking the engine', () => {
  const probe = () => {
    throw new Error('asked')
  }
  expect(canDecode('video/mp4', null, probe)).toBe(true)
  expect(canDecode('video/mp4', undefined, probe)).toBe(true)
})

it('refuses only on the empty answer', () => {
  expect(canDecode('video/mp4', 'hvc1', () => '')).toBe(false)
  expect(canDecode('video/mp4', 'hvc1', () => 'maybe')).toBe(true)
  expect(canDecode('video/mp4', 'hvc1', () => 'probably')).toBe(true)
})

it('asks the engine with the codecs string', () => {
  const asked: string[] = []
  canDecode('video/mp4', 'avc1', (t) => {
    asked.push(t)
    return 'probably'
  })
  expect(asked).toEqual(['video/mp4; codecs="avc1.42E01E"'])
})

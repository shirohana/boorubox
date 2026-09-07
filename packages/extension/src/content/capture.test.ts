// @vitest-environment jsdom
// @vitest-environment-options { "url": "https://page.test/gallery" }

import { beforeEach, describe, expect, it } from 'vitest'

import { captureImage, findImageElement } from './capture.js'

function pageWith(...sources: string[]) {
  document.body.innerHTML = sources.map((src) => `<img src="${src}">`).join('')
}

/** Stand in for the load jsdom never performs. */
function loaded(img: HTMLImageElement, width: number, height: number) {
  Object.defineProperty(img, 'complete', { value: true })
  Object.defineProperty(img, 'naturalWidth', { value: width })
  Object.defineProperty(img, 'naturalHeight', { value: height })
}

beforeEach(() => {
  document.body.innerHTML = ''
})

describe('findImageElement', () => {
  it('matches the clicked URL exactly', () => {
    pageWith('https://cdn.test/other.png', 'https://cdn.test/a.png')

    const found = findImageElement('https://cdn.test/a.png')

    expect(found?.src).toBe('https://cdn.test/a.png')
  })

  it('matches a URL that only agrees once normalised against the page', () => {
    pageWith('https://cdn.test/a.png')

    const found = findImageElement('//cdn.test/a.png')

    expect(found?.src).toBe('https://cdn.test/a.png')
  })

  it('matches when only the query string differs', () => {
    pageWith('https://cdn.test/a.png?cache=1')

    const found = findImageElement('https://cdn.test/a.png?cache=2')

    expect(found?.src).toBe('https://cdn.test/a.png?cache=1')
  })

  it('finds nothing when no image on the page is the one clicked', () => {
    pageWith('https://cdn.test/a.png')

    expect(findImageElement('https://cdn.test/b.png')).toBeNull()
  })

  it('does not raise on a clicked URL that will not parse', () => {
    pageWith('https://cdn.test/a.png')

    expect(findImageElement('::not a url::')).toBeNull()
  })
})

describe('captureImage', () => {
  it('answers with an error rather than raising when the image is not on the page', async () => {
    pageWith('https://cdn.test/a.png')

    const result = await captureImage('https://cdn.test/b.png')

    expect(result).toEqual({ error: 'Image not found in DOM' })
  })

  it('answers with an error rather than raising when the image has no dimensions', async () => {
    pageWith('https://cdn.test/a.png')
    // jsdom fetches nothing, so the element has to be told it finished; a
    // real capture only ever runs on an image the page already rendered.
    loaded(document.querySelector('img')!, 0, 0)

    const result = await captureImage('https://cdn.test/a.png')

    expect(result).toEqual({ error: 'Image has no dimensions' })
  })
})

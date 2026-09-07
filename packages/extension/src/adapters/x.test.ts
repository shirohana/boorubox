// @vitest-environment jsdom

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vitest'

import { extractPageContext } from './index.js'
import { x } from './x.js'

const IMAGE_URL = 'https://pbs.twimg.com/media/HRcOPAjbUAA-0TZ?format=jpg&name=4096x4096'
const PAGE_URL = 'https://x.com/IV70311741/status/2096160091699552568/photo/1'

function fixture(name: string): Document {
  const path = resolve(process.cwd(), 'src/adapters/__fixtures__', name)
  return new DOMParser().parseFromString(readFileSync(path, 'utf8'), 'text/html')
}

function extract(name: string, overrides: { imageUrl?: string, pageUrl?: string } = {}) {
  return x.extract({
    document: fixture(name),
    hostname: 'x.com',
    imageUrl: overrides.imageUrl ?? IMAGE_URL,
    pageUrl: overrides.pageUrl ?? PAGE_URL,
  })
}

function pageOf(document: Document, imageUrl = IMAGE_URL) {
  return extractPageContext({ document, hostname: 'x.com', imageUrl, pageUrl: PAGE_URL }, [x])
}

it('reads the handle, the post, its text and the original media', () => {
  expect(extract('x-post.html')).toEqual({
    handle: 'IV70311741',
    displayName: 'イブ',
    postUrl: 'https://x.com/IV70311741/status/2096160091699552568',
    postText: '夜の教室',
    originalUrl: 'https://pbs.twimg.com/media/HRcOPAjbUAA-0TZ?format=jpg&name=orig',
  })
})

it('leaves the text out when the post carried only media', () => {
  const fields = extract('x-post-no-text.html', {
    imageUrl: 'https://pbs.twimg.com/media/HRQzDY0bkAEIBS4?format=jpg&name=4096x4096',
    pageUrl: 'https://x.com/Rosu_art/status/2095355989281386971/photo/1',
  })

  expect(fields.postText).toBeUndefined()
  expect(fields.handle).toBe('Rosu_art')
  expect(fields.postUrl).toBe('https://x.com/Rosu_art/status/2095355989281386971')
  expect(fields.originalUrl).toBe(
    'https://pbs.twimg.com/media/HRQzDY0bkAEIBS4?format=jpg&name=orig',
  )
})

it('sends the fields that survive when the author block is gone', () => {
  const document = fixture('x-post.html')
  document.querySelector('[data-testid="User-Name"]')!.remove()

  const record = pageOf(document).record

  // The permalink anchor is not inside the author block, so the handle survives
  // it; this is the shape a partial rot has (spec: "Markup changed").
  expect(record).toEqual({
    site: 'x',
    fields: {
      handle: 'IV70311741',
      postUrl: 'https://x.com/IV70311741/status/2096160091699552568',
      postText: '夜の教室',
      originalUrl: 'https://pbs.twimg.com/media/HRcOPAjbUAA-0TZ?format=jpg&name=orig',
    },
  })
})

it('sends nothing but the site when the post markup is gone entirely', () => {
  const document = fixture('x-post.html')
  document.querySelector('article[data-testid="tweet"]')!.remove()

  const record = pageOf(document, 'https://example.test/not-media.png').record

  expect(record).toEqual({ site: 'x', fields: {} })
})

it('answers for the post the image is in, not the first one on the page', () => {
  // A timeline: the captured image sits inside its own article, and another
  // post is rendered above it.
  const document = new DOMParser().parseFromString(`
    <article data-testid="tweet">
      <a role="link" href="/someoneelse/status/111"><time datetime="2026-01-01"></time></a>
      <div data-testid="tweetText">not this one</div>
    </article>
    <article data-testid="tweet">
      <a role="link" href="/alice/status/222"><time datetime="2026-01-02"></time></a>
      <div data-testid="tweetText">this one</div>
      <img src="https://pbs.twimg.com/media/ABC123?format=jpg&name=small">
    </article>
  `, 'text/html')

  const fields = x.extract({
    document,
    hostname: 'x.com',
    imageUrl: 'https://pbs.twimg.com/media/ABC123?format=jpg&name=small',
    pageUrl: 'https://x.com/home',
  })

  expect(fields.handle).toBe('alice')
  expect(fields.postText).toBe('this one')
  expect(fields.originalUrl).toBe('https://pbs.twimg.com/media/ABC123?format=jpg&name=orig')
})

it('recognises x.com, twitter.com and their subdomains', () => {
  for (const hostname of ['x.com', 'www.x.com', 'twitter.com', 'mobile.twitter.com']) {
    expect(extractPageContext({ document: fixture('x-post.html'), hostname,
      imageUrl: IMAGE_URL, pageUrl: PAGE_URL }, [x]).record?.site).toBe('x')
  }
})

// X leaves `document.title` on the screen the user came from when a photo is
// opened straight off a timeline, so every title here is built from the post.
it('names the page after the post, not after the tab', () => {
  const document = fixture('x-post.html')
  document.title = '首頁 / X'

  expect(pageOf(document).pageTitle).toBe('イブ (@IV70311741) on X: 夜の教室')
})

it('names a post that carried only media by its author', () => {
  const document = fixture('x-post-no-text.html')
  const imageUrl = 'https://pbs.twimg.com/media/HRQzDY0bkAEIBS4?format=jpg&name=4096x4096'

  expect(pageOf(document, imageUrl).pageTitle).toBe('ロさん (@Rosu_art) on X')
})

it('falls back through the author block to the handle, then to the tab title', () => {
  const withoutName = fixture('x-post.html')
  withoutName.title = '首頁 / X'
  withoutName.querySelector('[data-testid="User-Name"]')!.remove()
  expect(pageOf(withoutName).pageTitle).toBe('@IV70311741 on X: 夜の教室')

  const withoutPost = fixture('x-post.html')
  withoutPost.title = '首頁 / X'
  withoutPost.querySelector('article[data-testid="tweet"]')!.remove()
  expect(pageOf(withoutPost).pageTitle).toBe('首頁 / X')
})

it('takes the first line of a long post and marks the cut', () => {
  const said = `${'あ'.repeat(140)}\nand a second line`
  const document = new DOMParser().parseFromString(`
    <article data-testid="tweet">
      <div data-testid="User-Name"><a role="link" href="/alice"><div dir="ltr">Alice</div></a></div>
      <a role="link" href="/alice/status/222"><time datetime="2026-01-02"></time></a>
      <div data-testid="tweetText">${said}</div>
      <img src="https://pbs.twimg.com/media/ABC123?format=jpg&name=small">
    </article>
  `, 'text/html')

  const title = pageOf(document, 'https://pbs.twimg.com/media/ABC123?format=jpg&name=small')
    .pageTitle

  expect(title).toBe(`Alice (@alice) on X: ${'あ'.repeat(100)}…`)
})

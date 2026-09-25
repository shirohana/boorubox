// @vitest-environment jsdom

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vitest'

import { extractPageContext } from './index.js'
import { pixiv } from './pixiv.js'

const PAGE_URL = 'https://www.pixiv.net/artworks/117090067'
const ORIGINAL = 'https://i.pximg.net/img-original/img/2024/03/20/17/08/26/117090067_p0.jpg'
const MASTER
  = 'https://i.pximg.net/img-master/img/2024/03/20/17/08/26/117090067_p0_master1200.jpg'

function fixture(): Document {
  const path = resolve(process.cwd(), 'src/adapters/__fixtures__/pixiv-artwork.html')
  return new DOMParser().parseFromString(readFileSync(path, 'utf8'), 'text/html')
}

function extract(imageUrl: string, document: Document = fixture()) {
  return pixiv.extract({ document, hostname: 'www.pixiv.net', imageUrl, pageUrl: PAGE_URL })
}

it('reads the artist, the work, its title and the original', () => {
  expect(extract(ORIGINAL)).toEqual({
    artist: 'かにビーム',
    userId: '3439325',
    workId: '117090067',
    title: '春の袴アロナ',
    originalUrl: ORIGINAL,
  })
})

it('finds the original when what was captured is the master image', () => {
  // The master names the page but not the extension — masters are always .jpg —
  // so the page's own full-size link is what supplies it.
  expect(extract(MASTER).originalUrl).toBe(ORIGINAL)
})

it('answers with the captured page of a multi-image work, not the first', () => {
  const second = MASTER.replace('_p0_', '_p1_')

  expect(extract(second).originalUrl).toBe(
    'https://i.pximg.net/img-original/img/2024/03/20/17/08/26/117090067_p1.jpg',
  )

  const thumbnail = 'https://i.pximg.net/c/250x250_80_a2/img-master/img/2024/03/20/17/08/26/'
    + '117090067_p3_square1200.jpg'
  expect(extract(thumbnail).originalUrl).toBe(
    'https://i.pximg.net/img-original/img/2024/03/20/17/08/26/117090067_p3.jpg',
  )
})

it('sends the fields that survive when the title is gone', () => {
  const document = fixture()
  document.querySelector('main h1')!.remove()
  document.querySelector('meta[property="twitter:title"]')!.remove()

  const record = extractPageContext(
    { document, hostname: 'www.pixiv.net', imageUrl: ORIGINAL, pageUrl: PAGE_URL },
    [pixiv],
  ).record

  expect(record).toEqual({
    site: 'pixiv',
    fields: {
      artist: 'かにビーム',
      userId: '3439325',
      workId: '117090067',
      originalUrl: ORIGINAL,
    },
  })
})

it('still names the work when only the address is left', () => {
  const document = new DOMParser().parseFromString('<main></main>', 'text/html')

  expect(extract(ORIGINAL, document)).toEqual({
    artist: undefined,
    userId: undefined,
    workId: '117090067',
    title: undefined,
    originalUrl: ORIGINAL,
  })
})

it('falls back to the avatar anchor for the id when the h2 name anchor is gone', () => {
  // The h2's own `/users/` anchor is gone entirely, so the primary selector
  // (`main h2 a[href^="/users/"]`) finds nothing; the avatar anchor lives
  // outside the h2 (as it does in the recommendations strip the module doc
  // warns about), so only the `:has()` fallback can find it.
  const document = new DOMParser().parseFromString(
    '<main><h2></h2><a href="/users/3439325">'
    + '<img src="https://i.pximg.net/user-profile/img/x.jpg"></a></main>',
    'text/html',
  )

  expect(extract(ORIGINAL, document).userId).toBe('3439325')
})

it('yields no userId when every users anchor is gone', () => {
  const document = fixture()
  for (const anchor of document.querySelectorAll('a[href^="/users/"]')) {
    anchor.remove()
  }

  expect(extract(ORIGINAL, document).userId).toBeUndefined()
})

it('recognises pixiv.net and its subdomains', () => {
  for (const hostname of ['pixiv.net', 'www.pixiv.net']) {
    expect(extractPageContext(
      { document: fixture(), hostname, imageUrl: ORIGINAL, pageUrl: PAGE_URL },
      [pixiv],
    ).record?.site).toBe('pixiv')
  }
})

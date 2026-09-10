import { expect, it } from 'vitest'
import { img } from '$lib/domain/image-fixture'
import { checkUploadForm, prefillUploadForm } from './upload-form'

it('fills the form from the image, tags in the library order', () => {
  const values = prefillUploadForm(img({
    tags: ['zebra', 'Apple', 'blue_sky'],
    rating: 's',
    pageUrl: 'https://x.com/alice/status/1',
    pageTitle: 'a drawing',
  }))

  expect(values).toEqual({
    tags: 'Apple blue_sky zebra',
    rating: 's',
    source: 'https://x.com/alice/status/1',
    artist: 'alice',
    commentaryTitle: 'a drawing',
    commentaryBody: '',
  })
})

it('preselects no rating for an unrated image, and refuses to send without one', () => {
  const values = prefillUploadForm(img({ tags: ['1girl'], rating: null }))

  expect(values.rating).toBeNull()
  expect(checkUploadForm(values)).toEqual({ ok: false, refusal: 'This upload needs a rating.' })
})

it('falls back to the image address when there is no page, and proposes no artist', () => {
  const values = prefillUploadForm(img({
    pageUrl: null,
    imageUrl: 'https://cdn.example.test/a.png',
  }))

  expect(values.source).toBe('https://cdn.example.test/a.png')
  expect(values.artist).toBe('')
})

it('leaves the source empty for an image with no address at all', () => {
  expect(prefillUploadForm(img({ pageUrl: null, imageUrl: null })).source).toBe('')
})

it('names both missing values when neither is given', () => {
  const values = prefillUploadForm(img({ tags: [], rating: null }))

  expect(checkUploadForm(values)).toEqual({
    ok: false,
    refusal: 'This upload needs at least one tag and a rating.',
  })
})

it('names the tags when only they are missing', () => {
  const values = prefillUploadForm(img({ tags: [], rating: 'e' }))

  expect(checkUploadForm(values)).toEqual({
    ok: false,
    refusal: 'This upload needs at least one tag.',
  })
})

it('sends the edited values, tags split and every text field trimmed', () => {
  const checked = checkUploadForm({
    tags: '  1girl   blue_sky ',
    rating: 'q',
    source: ' https://example.test/p ',
    artist: ' pixiv_user_1 ',
    commentaryTitle: ' a title ',
    commentaryBody: ' a body ',
  })

  expect(checked).toEqual({
    ok: true,
    form: {
      tags: ['1girl', 'blue_sky'],
      rating: 'q',
      source: 'https://example.test/p',
      artist: 'pixiv_user_1',
      commentaryTitle: 'a title',
      commentaryBody: 'a body',
    },
  })
})

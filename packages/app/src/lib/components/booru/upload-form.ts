// What the upload dialog opens with and what it is allowed to send (design
// D10). Pure, because the dialog itself has no test harness in this repo: the
// prefill rules and the two refusals are the parts worth pinning.

import type { BooruUploadForm, ImageRecord, Rating } from '@boorubox/shared'
import { extractArtistFromUrl } from '$lib/domain/artist-from-url'
import { sortTags, tagList } from '$lib/domain/tag-utils'

/**
 * The form's own editable state. `rating` is `Rating | null` here and never on
 * `BooruUploadForm`: an unrated image opens with nothing preselected (design
 * D10, reversing the legacy's `q` default) and the check below is what turns
 * one into the other.
 */
export interface UploadFormValues {
  /** Space-separated, as the field holds it. */
  tags: string
  rating: Rating | null
  source: string
  artist: string
  commentaryTitle: string
  commentaryBody: string
}

export type UploadFormCheck
  = | { ok: true, form: BooruUploadForm }
  /** What is missing, named (spec: "SHALL say which is missing"). */
    | { ok: false, refusal: string }

/** Design D10's table, and nothing else: every value here is editable after. */
export function prefillUploadForm(image: ImageRecord): UploadFormValues {
  // FIXME(booru-upload D11): `image.adapter` carries the capturing adapter's own
  // artist field (X → handle, Pixiv → artist), which beats a regular expression
  // over the address — but only for images captured after `bridge-extension`,
  // so the rules below are needed either way. Preferring the adapter field
  // belongs in a change that can test it against real captures.
  const candidate = extractArtistFromUrl(image.pageUrl ?? '')
  return {
    tags: sortTags(image.tags).join(' '),
    rating: image.rating,
    // Never a path on this machine for a local import: no booru could reach it
    // and it would put the library's location on a public post (design D3).
    source: image.pageUrl ?? image.imageUrl ?? '',
    artist: candidate.artist ?? '',
    commentaryTitle: image.pageTitle ?? '',
    commentaryBody: '',
  }
}

/**
 * The one place the send-time rules live: the refusal and the
 * `BooruUploadForm` come out of the same check, so what is refused and what is
 * sent cannot disagree. Rust checks the same two things again
 * (`AppError::BadRequest`); this is what makes sure it never has to.
 */
export function checkUploadForm(values: UploadFormValues): UploadFormCheck {
  const tags = tagList(values.tags)
  const { rating } = values
  if (tags.length === 0 || rating === null) {
    const missing = [
      ...(tags.length === 0 ? ['at least one tag'] : []),
      ...(rating === null ? ['a rating'] : []),
    ]
    return { ok: false, refusal: `This upload needs ${missing.join(' and ')}.` }
  }

  return {
    ok: true,
    form: {
      tags,
      rating,
      source: values.source.trim(),
      // Rust folds this into the tag string; the field stays separate here
      // because it is the one the prefill fills (design D10, D11).
      artist: values.artist.trim(),
      commentaryTitle: values.commentaryTitle.trim(),
      commentaryBody: values.commentaryBody.trim(),
    },
  }
}

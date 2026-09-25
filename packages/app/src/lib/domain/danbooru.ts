// Where a tag's context menu sends the user to look the tag up on Danbooru
// (`menu-polish` design D2): a Danbooru-style tagger sometimes needs a tag's
// wiki page, and an artist's own tag often differs from the name the artist
// is searched by, so an artist tag gets the artist search instead. The host
// is a constant here and nowhere else — the owner tags Danbooru-style and
// the self-hosted booru's wiki is not the reference (proposal non-goals).

import type { TagCategory } from '@boorubox/shared'

const DANBOORU_HOST = 'https://danbooru.donmai.us'

/** The menu label and the address `danbooruLookup` sends the browser to. */
export interface DanbooruLookup {
  label: string
  url: string
}

export function danbooruLookup(name: string, category: TagCategory): DanbooruLookup {
  const encoded = encodeURIComponent(name)
  if (category === 'artist') {
    return {
      label: 'Search artist on Danbooru',
      url: `${DANBOORU_HOST}/artists?commit=Search&search%5Bany_name_matches%5D=${encoded}&search%5Border%5D=created_at`,
    }
  }
  return {
    label: 'Open Danbooru wiki',
    url: `${DANBOORU_HOST}/wiki_pages/${encoded}`,
  }
}

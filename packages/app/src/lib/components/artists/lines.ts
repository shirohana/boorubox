// One splitter for the "one per line" textareas an artist entry uses
// (`RenameArtistDialog.svelte`'s URLs field, `ArtistForm.svelte`'s URLs
// field): both trimmed the same way and dropped the same blanks, so this is
// the one definition of what a line is rather than two copies drifting apart.

/** A textarea's text as its non-blank lines, each trimmed. */
export function lines(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
}

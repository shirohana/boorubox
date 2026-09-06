// Test-only builder: every api test needs a full LibraryStatus to override one
// field of. Nothing outside *.test.ts should import it.

import type { LibraryStatus } from '@boorubox/shared'
import { DEFAULT_PORT } from '@boorubox/shared'

export function status(overrides: Partial<LibraryStatus> = {}): LibraryStatus {
  return {
    opened: true,
    libraryPath: '/library',
    missingPath: null,
    imageCount: 0,
    listener: { running: true, port: DEFAULT_PORT, error: null },
    version: '0.1.0',
    ...overrides,
  }
}

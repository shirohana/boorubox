## ADDED Requirements

### Requirement: A video in the full-size view
Where the full-size view would show an image, it SHALL play a video: fitted to the whole of the
view's space without cropping, looping, starting on its own, with its sound off, and with the
platform's own playback controls shown so the user can pause, seek and turn the sound on. The
view SHALL NOT zoom a video: a click on it goes to its controls, and the wheel and a pinch
leave it at the fit. Every key the view binds SHALL act as it does for an image — the arrows
move, Escape and Space close, the inspector toggles. Moving to another item SHALL start it from
its beginning. A video the machine's webview cannot decode SHALL be shown as a plain message
saying so, in place of the picture, and the keys SHALL keep working.

#### Scenario: Opening a video
- **WHEN** a video's tile is activated
- **THEN** the view opens with the video playing from the start, looping, muted, fitted to the window, with the platform controls visible

#### Scenario: Space closes
- **WHEN** Space is pressed while a video is playing in the view
- **THEN** the view closes, as it does over an image

#### Scenario: A codec the webview lacks
- **WHEN** the view opens a file whose codec this machine's webview cannot decode
- **THEN** a message saying the video cannot be played here is shown where the picture would be, and the arrows still move to the neighbours

### Requirement: Video tiles
A video's tile SHALL show its first frame as its thumbnail and SHALL carry a badge with the
duration as minutes and seconds, so a video is told from an image at a glance. A video whose
thumbnail is not available SHALL show the badge over the tile's no-preview state.

#### Scenario: A video tile
- **WHEN** the grid shows an 8-second video
- **THEN** its tile shows the first frame and a `0:08` badge, and the tile's other badges (rating, posted, collections) are where they are on an image

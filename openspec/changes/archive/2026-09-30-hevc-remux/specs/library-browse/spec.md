## MODIFIED Requirements

### Requirement: A video in the full-size view
Where the full-size view would show an image, it SHALL play a video: fitted to the whole of the
view's space without cropping, looping, starting on its own, with its sound off, and with the
platform's own playback controls shown so the user can pause, seek and turn the sound on. The
view SHALL NOT zoom a video: a click on it goes to its controls, and the wheel and a pinch
leave it at the fit. Every key the view binds SHALL act as it does for an image — the arrows
move, Escape and Space close, the inspector toggles. Moving to another item SHALL start it from
its beginning. Before playing a video whose codec is recorded, the view SHALL ask the browser
engine whether it decodes that codec; when the engine says it does not, the view SHALL show a
plain message naming the codec in place of the picture, SHALL NOT start the video (not even
its sound), and on Windows SHALL add that the system's browser engine plays HEVC only with
Microsoft's HEVC Video Extensions; the keys SHALL keep working. A video the engine accepted
and then cannot decode SHALL be shown as a plain message saying so, in place of the picture,
and the keys SHALL keep working.

#### Scenario: Opening a video
- **WHEN** a video's tile is activated
- **THEN** the view opens with the video playing from the start, looping, muted, fitted to the window, with the platform controls visible

#### Scenario: Space closes
- **WHEN** Space is pressed while a video is playing in the view
- **THEN** the view closes, as it does over an image

#### Scenario: A codec the engine says it cannot decode
- **WHEN** the view opens an HEVC clip on a machine whose browser engine answers that it does not decode HEVC
- **THEN** a message naming HEVC is shown where the picture would be, no sound plays, and the arrows still move to the neighbours

#### Scenario: A codec the webview lacks
- **WHEN** the view opens a file whose codec this machine's webview cannot decode
- **THEN** a message saying the video cannot be played here is shown where the picture would be, and the arrows still move to the neighbours

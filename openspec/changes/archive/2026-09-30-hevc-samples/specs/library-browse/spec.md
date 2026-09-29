## MODIFIED Requirements

### Requirement: A video in the full-size view
Where the full-size view would show an image, it SHALL play a video: fitted to the whole of the
view's space without cropping, looping, starting on its own, with its sound off, and with the
platform's own playback controls shown so the user can pause, seek and turn the sound on. The
view SHALL NOT zoom a video: a click on it goes to its controls, and the wheel and a pinch
leave it at the fit. Every key the view binds SHALL act as it does for an image — the arrows
move, Escape and Space close, the inspector toggles. Moving to another item SHALL start it from
its beginning. Before playing a video whose codec is recorded, the view SHALL ask the browser
engine whether it decodes that codec; when the engine says it does not, the view SHALL say, in
place of the picture, that it is converting the video for playback because this machine's
browser engine cannot decode that codec (naming it) and that the user should wait, SHALL show
the conversion's progress, SHALL NOT start the original (not even its sound), and SHALL play
the converted copy when it is ready; on Windows the message SHALL add that the system's
browser engine plays HEVC only with Microsoft's HEVC Video Extensions. A conversion that fails
SHALL be shown as a plain message with the tool's reason. The keys SHALL keep working
throughout, and moving to another item SHALL show that item at once. A video the engine
accepted and then cannot decode SHALL be shown as a plain message saying so, in place of the
picture, and the keys SHALL keep working.

#### Scenario: Opening a video
- **WHEN** a video's tile is activated
- **THEN** the view opens with the video playing from the start, looping, muted, fitted to the window, with the platform controls visible

#### Scenario: Space closes
- **WHEN** Space is pressed while a video is playing in the view
- **THEN** the view closes, as it does over an image

#### Scenario: A codec the engine says it cannot decode
- **WHEN** the view opens an HEVC clip on a machine whose browser engine answers that it does not decode HEVC
- **THEN** the converting message naming HEVC and a progress bar are shown where the picture would be, no sound plays, the arrows still move to the neighbours, and when the conversion ends the copy plays looping and muted

#### Scenario: A codec the webview lacks
- **WHEN** the view opens a file whose codec this machine's webview cannot decode
- **THEN** a message saying the video cannot be played here is shown where the picture would be, and the arrows still move to the neighbours

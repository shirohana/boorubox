## ADDED Requirements

### Requirement: The tag editor belongs to the image it was opened for
When the image the inspector describes changes while the tag editor is open — another card
focused or selected, another image in the full-size viewer, or the rows moving under the panel
— the editor SHALL close and its text SHALL be discarded, and nothing SHALL be written to
either image. The typed text was for the image that was on screen when it was typed; saving it
onto the image that replaced it overwrites the wrong image's tags without a trace (owner,
2026-09-28). A write that changes the same image — its own save, a rating, a facts save, a
capture refreshing its record — SHALL NOT close the editor or change its text.

#### Scenario: Another card while editing
- **WHEN** the user opens the tag editor of image A, types `dog`, and clicks the card of image B
- **THEN** the inspector shows image B's tags read-only, no editor is on screen, and neither A nor B has gained `dog`

#### Scenario: Another image in the viewer
- **WHEN** the user opens the tag editor in the viewer's inspector, types `dog`, and presses `→`
- **THEN** the next image is shown with its tags read-only, and neither image has gained `dog`

#### Scenario: The same image changed
- **WHEN** the user has the tag editor of image A open with `dog` typed, and image A's record changes (a rating is chosen, or its facts are saved)
- **THEN** the editor is still open and still reads `dog` where it was typed

## MODIFIED Requirements

### Requirement: The rating can be set without opening the image
The app SHALL offer the same five choices from a context menu on a thumbnail in the grid, so
a rating can be given to an image the user is not currently inspecting. The five choices SHALL
be drawn as one row of pills in the rating colours the inspector's control uses, the image's
current rating filled and the others dimmed, so the current rating is read without reading
(owner, 2026-09-25).

#### Scenario: From the grid
- **WHEN** the user opens the context menu of a thumbnail and picks `q`
- **THEN** that image is rated `q`, the grid shows it, and the inspector's current image is unchanged

#### Scenario: The current rating is the filled pill
- **WHEN** the user opens the context menu of a thumbnail rated `s`
- **THEN** the `s` pill is filled in its colour and `g`, `q`, `e` and `none` are outlined

## MODIFIED Requirements

### Requirement: Pixiv context
On a Pixiv artwork page the adapter SHALL extract the artist, the artist's user id (the digits
of the artist link's `/users/<id>` path, the one fact about the artist that survives a rename),
the work id, the work title and the original-resolution URL of the captured image.

#### Scenario: Capture from an artwork page
- **WHEN** the user captures an image on an artwork page
- **THEN** the record names the site and carries the artist, user id, work id, title and original-resolution URL

#### Scenario: Multi-image work
- **WHEN** the work holds several images and the user captures the second
- **THEN** the original-resolution URL is the one for the captured image, not for the first

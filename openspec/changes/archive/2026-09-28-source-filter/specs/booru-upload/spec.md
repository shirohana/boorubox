## MODIFIED Requirements

### Requirement: The form is prefilled from the image
The upload form SHALL open prefilled with: the image's tags in the library's tag order; the
image's rating; a source address, being the image's source URL as the library reports it — the
page the image was captured from, or the image's own address when no page address is known,
the same value the `source:` search metatag matches; an artist candidate derived from the page
address for the sites whose address form is known, empty otherwise; and an optional commentary
title and body, with the title prefilled from the page title. Every prefilled value SHALL be
editable before sending, and edits SHALL NOT change the image in the library.

#### Scenario: Captured image
- **WHEN** the form opens for an image captured from a page
- **THEN** tags, rating, the page address as source, an artist candidate where the page address
  is one of the known forms, and the page title as the commentary title are filled in

#### Scenario: Imported file with no page
- **WHEN** the form opens for an image with no page address
- **THEN** the source is the image's own address if it has one and is otherwise empty, and the
  artist candidate is empty

#### Scenario: Edits stay in the form
- **WHEN** the user changes tags or rating in the form and sends the upload
- **THEN** the values sent are the edited ones and the image's own tags and rating are unchanged

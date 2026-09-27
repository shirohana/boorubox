## ADDED Requirements

### Requirement: The view stays on its image when the result is re-read
While the full-size view is open, a re-read of the same search — a capture stored, an import
finished, a write made from the view or its inspector — SHALL leave the view on the image it was
showing, at that image's row in the re-read result, and its previous/next order SHALL be the
re-read result's. When that image is no longer in the result, the view SHALL stay at the row it
was on, showing the image that now fills it. The re-read SHALL NOT close the view.

#### Scenario: A capture lands while viewing
- **WHEN** the view is open on the second image of a result in capture order and a capture is stored
- **THEN** the view still shows the same image, the left arrow moves to the image that was first, and closing returns focus to that image's thumbnail at its new row

#### Scenario: An artist renamed from inside the view
- **WHEN** the result is ordered by last update, the view is open in inspect mode, and the user renames the shown image's artist from the panel
- **THEN** the view still shows the same image after the result is re-read

#### Scenario: The viewed image is trashed
- **WHEN** the view is open on an image that is not the last of the result and the user moves it to the trash
- **THEN** the view stays at that row and shows the image that now fills it

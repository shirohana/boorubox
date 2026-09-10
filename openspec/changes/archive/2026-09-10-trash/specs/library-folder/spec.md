## MODIFIED Requirements

### Requirement: External file changes never crash the app
An image whose file was removed or renamed outside the app SHALL be shown as missing and
SHALL offer to move its record to the trash, from where the record can be restored if the file
comes back; the rest of the library SHALL keep working. No action offered on a missing image
SHALL destroy its record outright.

#### Scenario: File removed externally
- **WHEN** an image file under `images/` is deleted outside the app
- **THEN** its card shows a missing state, the grid still renders, and the user can move the record to the trash

#### Scenario: File restored
- **WHEN** a file previously marked missing reappears at its path
- **THEN** the image renders again and the missing state is cleared

#### Scenario: File restored after the record was trashed
- **WHEN** the user moved a missing image's record to the trash and the file later reappears
- **THEN** restoring the record from the trash brings the image back into the library and it renders again

# booru-upload Specification

## Purpose
Posting one image from the library to a configured booru: a form prefilled from what the library
already knows, a sequence of remote steps whose failures are told apart, and a record of the post
written only once the post exists.

## Requirements

### Requirement: An image is uploaded from its Inspector
The app SHALL offer an upload action for the image shown in the Inspector when at least one booru
site is configured, and SHALL open a form for the chosen site before anything is sent. When no
site is configured, the app SHALL NOT show an upload action; it SHALL point the user at the place
where sites are configured. When more than one site is configured, the action SHALL let the user
choose which one.

#### Scenario: No sites configured
- **WHEN** the Inspector shows an image and no booru site is configured
- **THEN** no upload action is offered, and the settings section for booru sites is named as
  where to add one

#### Scenario: One site configured
- **WHEN** the Inspector shows an image and exactly one site is configured
- **THEN** the action names that site and opens the upload form for it

#### Scenario: Several sites configured
- **WHEN** two or more sites are configured
- **THEN** the action lets the user pick the site, and opens the form for the one picked

### Requirement: The form is prefilled from the image
The upload form SHALL open prefilled with: the image's tags in the library's tag order; the
image's rating; a source address, being the page the image was captured from, or the image's own
address when no page address is known; an artist candidate derived from the page address for the
sites whose address form is known, empty otherwise; and an optional commentary title and body,
with the title prefilled from the page title. Every prefilled value SHALL be editable before
sending, and edits SHALL NOT change the image in the library.

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

### Requirement: An upload will not be sent without tags and a rating
The app SHALL refuse to send an upload whose tag field is empty or whose rating is unset, and
SHALL say which is missing. The app SHALL NOT choose a rating on the user's behalf.

#### Scenario: Unrated image
- **WHEN** the form opens for an image with no rating
- **THEN** no rating is preselected, and sending is refused until one is chosen

#### Scenario: No tags
- **WHEN** the tag field is empty
- **THEN** sending is refused with a message naming the tags as missing

### Requirement: The image's own bytes are what is posted
The app SHALL send the file held in the library as the content of the upload. It SHALL NOT ask
the booru to fetch the image from its original address.

#### Scenario: Origin blocks the booru
- **WHEN** the image came from a site that refuses requests from other hosts
- **THEN** the upload still succeeds, because the booru is never asked to fetch anything

#### Scenario: Post matches the library
- **WHEN** an upload succeeds
- **THEN** the posted file is the same bytes the library holds for that image

### Requirement: Each step of the upload is reported separately
The upload SHALL proceed as: authenticate, hand the file to the booru, wait for the booru to
finish processing it, create the post, and — only if a commentary was given — set the artist
commentary. When a step fails, the app SHALL report which step failed and the reason the booru
gave, and SHALL leave the library unchanged, except that a failure of the commentary step alone
SHALL still count as a successful post.

#### Scenario: Credential rejected
- **WHEN** the booru rejects the credential
- **THEN** the failure names authentication as the step, and nothing is recorded against the image

#### Scenario: Booru refuses the file
- **WHEN** the booru rejects the file, for example as an unsupported type
- **THEN** the failure names the upload step and repeats the booru's own message, and nothing is
  recorded against the image

#### Scenario: Booru already holds the file
- **WHEN** a post on the booru already holds a file with the same checksum as the library's
- **THEN** the upload is refused before the file is sent, the failure names the upload step and
  the existing post, and nothing is recorded against the image or changed on that post

#### Scenario: Processing does not finish
- **WHEN** the booru has not finished processing the file within the app's waiting period
- **THEN** the failure says the upload was accepted but not finished, identifies the upload on the
  booru so the user can finish it there, and records nothing against the image

#### Scenario: Post creation fails
- **WHEN** the file was processed but the post could not be created
- **THEN** the failure names the post step, repeats the booru's message, and records nothing
  against the image

#### Scenario: Commentary fails
- **WHEN** the post was created and only the artist commentary call fails
- **THEN** the upload counts as successful, the post is recorded, and the commentary failure is
  shown as a warning naming the post

### Requirement: A successful upload is recorded against the image
On a successful upload the app SHALL record, against that image and that site, the remote post
identifier and the time of posting, in a single durable write. An image SHALL NOT show a record
for a post that was not created.

#### Scenario: Success
- **WHEN** the post is created
- **THEN** the image records that site, the post's identifier and the time, and the record
  survives restarting the app

#### Scenario: Interrupted before the post exists
- **WHEN** the sequence fails at any step before the post is created
- **THEN** the image has no record for that site

### Requirement: A slow or unreachable booru cannot wedge the app
Every request the upload makes SHALL have a time limit, and the whole waiting period for
processing SHALL be bounded. While an upload runs the rest of the app SHALL stay usable, and the
form SHALL show that the upload is running, naming the steps the sequence performs.

#### Scenario: Booru stops answering mid-upload
- **WHEN** the booru accepts the file and then stops answering
- **THEN** the wait ends within the bounded period with the processing-timeout failure, and the
  window was usable throughout

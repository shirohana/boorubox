## Purpose

Extra source context that only the browser can see — who posted the image, on what work, at
what original URL — pulled from known sites at capture time and handed to the app verbatim,
so the app can apply policy to it later without ever visiting the page.

## ADDED Requirements

### Requirement: Adapters produce a plain record and interpret nothing
An adapter SHALL produce a record naming the site it recognised and a flat map of extracted
fields whose values are text or lists of text. It SHALL NOT derive tags, ratings, artists or
any other library concept from what it extracted.

#### Scenario: Record sent with the capture
- **WHEN** a capture is made on a recognised site
- **THEN** the posted capture carries the site name and the extracted fields exactly as read from the page

#### Scenario: Unrecognised site
- **WHEN** a capture is made on a site no adapter recognises
- **THEN** no adapter record is sent and the capture carries only image URL, page URL, page title and time

### Requirement: The page names itself
A site may leave its own title naming the screen the user came from — X does this when a photo
is opened straight off a timeline — and the browser's tab record reports that same title, so
neither is trustworthy on its own. An adapter MAY therefore name the page from the fields it
already extracted; where it does, that name SHALL be used for every capture on that site, and
where it cannot, the page's own title as read at capture time SHALL be used. A page with no
adapter and no content script in it SHALL fall back to the tab's title rather than none.

The stored title is a display name. Nothing SHALL read meaning out of it; a field an uploader
or a rule needs SHALL come from the adapter record instead.

#### Scenario: Site left its title on the previous screen
- **WHEN** a capture is made on a page whose title still names the screen the user came from
- **THEN** the stored title names what is actually on the page, not what the tab reports

#### Scenario: No adapter title to be had
- **WHEN** the adapter cannot build a name from what it read
- **THEN** the page's own title is stored

### Requirement: X/Twitter context
On an X/Twitter status page the adapter SHALL extract the author's handle without its
leading marker, the author's display name, the canonical URL of the post, the post's text,
and the original-size URL of the captured media. It SHALL name the page after the post's
author and, where the post has one, its text.

#### Scenario: Capture from a post
- **WHEN** the user captures an image inside a post by an author
- **THEN** the record names the site and carries the handle, the display name, the post URL, the post text and the original-size media URL

#### Scenario: Post with no text
- **WHEN** the post carries only media and no text
- **THEN** the text field is absent, the other fields are still present, and the page is named after its author alone

### Requirement: Pixiv context
On a Pixiv artwork page the adapter SHALL extract the artist, the work id, the work title
and the original-resolution URL of the captured image.

#### Scenario: Capture from an artwork page
- **WHEN** the user captures an image on an artwork page
- **THEN** the record names the site and carries the artist, work id, title and original-resolution URL

#### Scenario: Multi-image work
- **WHEN** the work holds several images and the user captures the second
- **THEN** the original-resolution URL is the one for the captured image, not for the first

### Requirement: A failing adapter never blocks a capture
When a page's markup no longer matches what an adapter looks for, the adapter SHALL omit the
fields it could not read, or produce no record at all, and SHALL NOT prevent or delay the
capture. An adapter SHALL never abort a capture by raising.

#### Scenario: Markup changed
- **WHEN** the element an adapter reads a field from is absent
- **THEN** that field is omitted, the remaining fields are still sent, and the capture is delivered as usual

#### Scenario: Adapter raises
- **WHEN** an adapter raises while reading the page
- **THEN** the capture is still made and delivered, with no adapter record

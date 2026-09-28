## MODIFIED Requirements

### Requirement: An artist entry is a tag that owns profile URLs
The library SHALL keep artist entries: an artist tag and the list of URLs it owns, as Danbooru
keeps an artist's URLs. A URL SHALL have one owner; giving a URL another artist owns SHALL be
refused naming that artist. URLs SHALL be compared normalised — scheme ignored, host
lower-cased with a leading `www.`, `mobile.` or `m.` dropped and `twitter.com` read as `x.com`,
query and fragment dropped, trailing slashes dropped, path lower-cased — and an entry's URL
SHALL own a URL that equals it or continues it past a `/`. A URL with no host or a host without
a dot SHALL be refused as not a URL. A URL that is a host alone SHALL be refused, naming the
host and asking for the account's path, only when the host is one the app reads an account's
profile from by its path (`x.com`, `pixiv.net`); a host-only URL of any other site SHALL be
stored as its host and matched once the app reads that site. An entry naming a tag that exists
under a category other than artist SHALL be refused. Entries SHALL be stored with the library,
carried in its describing file, and restored on rebuild; a describing file written before
entries existed SHALL restore none.

#### Scenario: A prefix owns the post
- **WHEN** `metaljelly` owns `https://x.com/metaljelly0811`
- **THEN** it owns `https://twitter.com/MetalJelly0811/status/123?s=20` and `http://www.x.com/metaljelly0811/` and not `https://x.com/metaljelly08110`

#### Scenario: One owner per URL
- **WHEN** `alice` owns `https://x.com/alice_art` and the user gives that URL to `bob`
- **THEN** the write is refused naming `alice`, and both entries are unchanged

#### Scenario: Entries survive a rebuild
- **WHEN** `metaljelly` owns two URLs and the library is rebuilt from its folder
- **THEN** `metaljelly` owns the same two URLs afterwards

#### Scenario: A Fanbox profile, either spelling
- **WHEN** an entry is saved with `https://kanibiimu.fanbox.cc/` and `https://www.fanbox.cc/@kanibiimu`
- **THEN** it is saved and lists `kanibiimu.fanbox.cc` and `fanbox.cc/@kanibiimu`

#### Scenario: A bare X host
- **WHEN** an entry is saved with `x.com`
- **THEN** it is refused, the message names `x.com` and asks for the account's path

#### Scenario: A name is not a URL
- **WHEN** an entry is saved with `metaljelly`
- **THEN** it is refused as not looking like a URL

## ADDED Requirements

### Requirement: The artist dialog explains an unread site
The artist dialog SHALL say, beside the profile URLs, that a URL from a site the app does not
read yet is kept and matched once it does.

#### Scenario: Reading the hint
- **WHEN** the dialog is open
- **THEN** the hint is visible under the Profile URLs field

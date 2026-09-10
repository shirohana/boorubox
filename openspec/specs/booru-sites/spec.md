# booru-sites Specification

## Purpose
The boorus a library posts to: their addresses, the account used for each, and the credential
that authenticates it. Configuration lives with the library; the secret lives with the operating
system, so a copied or synced library folder never carries an API key.

## Requirements

### Requirement: A library holds a list of booru sites
The app SHALL let the user add, edit and remove booru sites, each with a display name, a base
address and a username, and SHALL store that list with the library so it travels with the
library folder. Two sites SHALL NOT share a base address. The list SHALL be reachable from the
settings screen.

#### Scenario: Adding a site
- **WHEN** the user adds a site with a name, a base address and a username
- **THEN** it appears in the list and is offered as an upload target for images in this library

#### Scenario: A second library
- **WHEN** the user switches to a library with no sites configured
- **THEN** the site list is empty, and the first library's sites are unaffected

#### Scenario: Duplicate address
- **WHEN** the user saves a site whose base address matches one already configured
- **THEN** the save is refused and the existing site is named in the message

### Requirement: The API key is held by the operating system
The app SHALL store each site's API key in the operating system's credential store, keyed so that
one entry belongs to exactly one site. It SHALL NOT write the key to the library file, to the
application settings file, or to any log or error message. Removing a site SHALL remove its
stored key.

#### Scenario: Key entered
- **WHEN** the user saves an API key for a site
- **THEN** the key is handed to the operating system's credential store and no copy of it exists
  in the library folder or in the application settings

#### Scenario: Library inspected
- **WHEN** the library file is opened by any other means
- **THEN** it contains the site's name, address and username, and no credential

#### Scenario: Site removed
- **WHEN** the user removes a configured site
- **THEN** its stored credential is removed as well, and posts already recorded against that site
  are kept

### Requirement: The credential store can refuse, and the app says so
When the operating system's credential store cannot be read or written — it is locked, the user
denies access, or the platform has none available — the app SHALL report that fact with the
reason it was given, SHALL keep the site's other settings saved, and SHALL NOT fall back to
storing the key anywhere else. A site whose credential cannot be read SHALL be shown as
unusable, and an upload to it SHALL be refused before any request is sent.

#### Scenario: Saving refused
- **WHEN** the credential store refuses to store a key
- **THEN** the site's name, address and username are still saved, and the failure and its reason
  are shown against that site

#### Scenario: Reading refused
- **WHEN** the credential for a site cannot be read
- **THEN** the site is marked unusable with the reason, and choosing it as an upload target
  reports the same reason instead of contacting the booru

### Requirement: Test connection reports what is wrong
Each configured site SHALL offer a connection test that contacts the site with the stored
credential and reports one of: the site answered and accepted the credential; the site answered
and rejected the credential; or the site could not be reached, with the transport's reason. The
test SHALL NOT create, modify or delete anything on the booru.

#### Scenario: Working site
- **WHEN** the user tests a site whose address and credential are correct
- **THEN** the result says the connection succeeded and the account was accepted

#### Scenario: Wrong credential
- **WHEN** the site answers but rejects the username or API key
- **THEN** the result names the credential as the problem, not the address

#### Scenario: Unreachable site
- **WHEN** the address does not resolve, refuses the connection, or does not answer in time
- **THEN** the result names the address as the problem and includes the reason reported by the
  transport

### Requirement: An insecure base address is flagged
When a site's base address is not a secure one, the app SHALL warn that the username and API key
will be sent over an unprotected connection, and SHALL still allow the site to be saved.

#### Scenario: Plain address on a local instance
- **WHEN** the user enters an insecure base address for a self-hosted instance
- **THEN** the form warns that credentials will travel unprotected and the site can still be saved

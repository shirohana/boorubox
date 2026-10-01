## MODIFIED Requirements

### Requirement: Settings is a set of pages with a nav
The settings screen SHALL be a set of pages, with a nav listing them at the left of the main
content region and the chosen page beside it. The nav SHALL list, in this order: General,
Library, Artists, Rules, Stamps, Booru, Keyboard, About, and SHALL show which page is open.
Each page SHALL have its own address, and navigating to that address from anywhere in the app
SHALL show that page with the nav, without passing through another page first. A page's
content SHALL be as wide as the settings screen's single column was; the nav SHALL NOT narrow
the app's own sidebar. When the main content region is too narrow for the nav and the page
side by side, the nav SHALL sit above the page instead. The nav SHALL stay in view while a long
page scrolls.

Opening Settings from the app's navigation SHALL show the settings page last shown on this
machine, surviving a restart, or General when none has been shown or the remembered one no
longer exists. The app's navigation SHALL show Settings as the current screen on every
settings page.

What each page holds is said by the capability that owns the fact; General holds the settings
set once and then left alone — how the app looks and how it listens for captures — because a
page holding a single field is not worth a place in the nav (owner, 2026-09-28).

A page row that pairs an action button with its description SHALL keep the button at the
right edge of the row whether or not the description has wrapped the button onto a line of
its own (owner, 2026-10-01: a button that lands left under a wrapped description reads as a
stray).

#### Scenario: The pages in order
- **WHEN** the user opens Settings
- **THEN** the nav lists General, Library, Artists, Rules, Stamps, Booru, Keyboard, About, and the page shown is marked in it

#### Scenario: First visit
- **WHEN** Settings is opened on a machine where no settings page has been shown before
- **THEN** the General page is shown

#### Scenario: The last page is remembered
- **WHEN** the user shows the Artists page, leaves Settings, quits and restarts the app, and opens Settings again
- **THEN** the Artists page is shown

#### Scenario: A link to a page
- **WHEN** another screen of the app navigates straight to the Artists page's address
- **THEN** the Artists page is shown with the nav, and no other settings page was shown on the way

#### Scenario: The sidebar on a settings page
- **WHEN** any settings page is shown
- **THEN** Settings is the entry marked current in the app's sidebar, and no other entry is

#### Scenario: A long page
- **WHEN** the Artists page lists more entries than fit and the user scrolls to the end
- **THEN** the nav is still on screen, and no other settings section is below the list

#### Scenario: A narrow window
- **WHEN** the main content region is narrower than the nav and a page need side by side
- **THEN** the nav sits above the page, and nothing scrolls sideways

#### Scenario: A wrapped action row
- **WHEN** the Library page is narrow enough that "Rebuild library index" no longer fits beside its description
- **THEN** the button sits on its own line, at the right edge

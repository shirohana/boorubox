//! Auto-tag rules (`auto-tag-rules`): matching a rule's pattern against what a
//! source said about an image (design D5, D6), the library's rule store
//! (design D1, D2, D4, D6), running the current rules over images already
//! stored (design D9), and moving rules between libraries as JSON (design
//! D10). `ingest::store_image` is the one caller that applies rules at write
//! time; this module only matches and stores, it never decides when to run.

use std::collections::{HashMap, HashSet};

use regex::{Regex, RegexBuilder};
use rusqlite::{Connection, Row, params};

use crate::db;
use crate::error::{AppError, Result};
use crate::library::{Library, SharedLibrary, with_library};
use crate::model::{
    Rule, RuleInput, RuleListEntry, RuleRunCount, RulesImportReport, RulesRunReport,
    SiteAdapterRecord,
};
use crate::tags;

// ---------------------------------------------------------------------------
// Matching (design D5, D6)
// ---------------------------------------------------------------------------

/// The texts a rule is matched against: the title, the adapter record's site,
/// and every string its fields carry, including the entries of a field
/// holding several values (design D5). Field names the app has no meaning for
/// are matched like any other, since `bridge-extension` stores the record
/// verbatim precisely so this is not a second definition of what an adapter
/// produces. The page address and the image address are never here: they are
/// long, punctuation-dense strings a short substring pattern matches inside by
/// accident, and every address that carries information is already an adapter
/// field (`postUrl`, `originalUrl`).
pub fn haystacks(title: Option<&str>, adapter: Option<&SiteAdapterRecord>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(title) = title
        && !title.is_empty()
    {
        out.push(title.to_string());
    }
    if let Some(adapter) = adapter {
        out.push(adapter.site.clone());
        push_field_strings(&adapter.fields, &mut out);
    }
    out
}

fn push_field_strings(fields: &serde_json::Value, out: &mut Vec<String>) {
    let serde_json::Value::Object(map) = fields else {
        return;
    };
    for value in map.values() {
        match value {
            serde_json::Value::String(text) => out.push(text.clone()),
            serde_json::Value::Array(items) => {
                for item in items {
                    if let serde_json::Value::String(text) = item {
                        out.push(text.clone());
                    }
                }
            }
            _ => {}
        }
    }
}

/// Compile `pattern` as the case-insensitive regular expression `matches`
/// tests with, or the engine's reason it cannot. `case_insensitive(true)` on
/// the builder rather than a prepended `(?i)`: prepending would corrupt a
/// pattern that legitimately starts with something else and would be a second
/// place the case rule is written (design D6).
fn compile_regex(pattern: &str) -> std::result::Result<Regex, String> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .map_err(|error| error.to_string())
}

/// Why a regular-expression rule's pattern cannot be used, or `None` — for a
/// plain-text rule, always `None`, since substring matching cannot be invalid
/// (design D6). Computed here, never stored: the engine is the only authority
/// on whether a pattern is usable, and a stored flag would be a cached answer
/// that goes stale the day the crate is upgraded.
pub fn pattern_error(rule: &Rule) -> Option<String> {
    if rule.is_regex {
        compile_regex(&rule.pattern).err()
    } else {
        None
    }
}

/// `matchesRule`, ported line for line (design D6): a disabled rule never
/// matches, an empty pattern matches everything, a plain-text pattern matches
/// case-insensitive substring on any haystack, and a regular expression
/// matches case-insensitively on any haystack. A pattern the engine cannot use
/// never matches — its reason is [`pattern_error`], read separately, so an
/// invalid rule costs the image nothing and never panics.
pub fn matches(rule: &Rule, haystacks: &[String]) -> bool {
    if !rule.enabled {
        return false;
    }
    if rule.pattern.is_empty() {
        return true;
    }
    if rule.is_regex {
        match compile_regex(&rule.pattern) {
            Ok(regex) => haystacks.iter().any(|haystack| regex.is_match(haystack)),
            Err(_) => false,
        }
    } else {
        let needle = rule.pattern.to_lowercase();
        haystacks
            .iter()
            .any(|haystack| haystack.to_lowercase().contains(&needle))
    }
}

/// `getAutoTags`, ported (design D6): the tags of every rule that matches,
/// unioned into a set in rule order so a tag named by two rules lands once.
pub fn auto_tags(rules: &[Rule], haystacks: &[String]) -> Vec<String> {
    union_tags(rules.iter().filter(|rule| matches(rule, haystacks)))
}

fn union_tags<'a>(rules: impl Iterator<Item = &'a Rule>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = Vec::new();
    for rule in rules {
        for tag in &rule.tags {
            if seen.insert(tag.clone()) {
                tags.push(tag.clone());
            }
        }
    }
    tags
}

// ---------------------------------------------------------------------------
// The store (design D1, D2, D4, D6)
// ---------------------------------------------------------------------------

const RULE_COLUMNS: &str =
    "id, name, pattern, is_regex, tags_json, enabled, created_at, updated_at";

fn row_to_rule(row: &Row) -> rusqlite::Result<Rule> {
    let tags_json: String = row.get(4)?;
    Ok(Rule {
        id: row.get(0)?,
        name: row.get(1)?,
        pattern: row.get(2)?,
        is_regex: row.get(3)?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        enabled: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

/// Every rule, ordered by name case-insensitively (design D4 — there is no
/// rule order, so the list is ordered for a human to scan), each carrying its
/// pattern's validity compiled at read time (design D6).
pub fn list(conn: &Connection) -> Result<Vec<RuleListEntry>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {RULE_COLUMNS} FROM rules ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map([], row_to_rule)?;
    let mut entries = Vec::new();
    for row in rows {
        let rule = row?;
        let pattern_error = pattern_error(&rule);
        entries.push(RuleListEntry {
            rule,
            pattern_error,
        });
    }
    Ok(entries)
}

/// The enabled rules, for the ingest path (design D7). Read fresh on every
/// call rather than cached in `AppState`: a cache would need invalidating from
/// `upsert`, `delete` and `import_json`, and the failure mode of missing one
/// is silent — images tagged by a rule the user just deleted.
pub fn enabled_rules(conn: &Connection) -> Result<Vec<Rule>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {RULE_COLUMNS} FROM rules WHERE enabled = 1"
    ))?;
    let rows = stmt.query_map([], row_to_rule)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn require_rule(conn: &Connection, id: &str) -> Result<Rule> {
    let mut stmt = conn.prepare(&format!("SELECT {RULE_COLUMNS} FROM rules WHERE id = ?1"))?;
    match stmt.query_row([id], row_to_rule) {
        Ok(rule) => Ok(rule),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::NotFound(format!("rule {id}"))),
        Err(error) => Err(error.into()),
    }
}

/// Create a rule when `input.id` is absent, or edit the one it names (design
/// D6). Refuses an empty name, an empty tag list, and — for a regular
/// expression the edit actually writes — a pattern the engine cannot use, so a
/// rule written in this app is valid by construction; [`import_json`] is the
/// one path that stores an invalid one instead.
pub fn upsert(library: &Library, input: &RuleInput) -> Result<Rule> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("a rule needs a name".to_string()));
    }
    if input.tags.is_empty() {
        return Err(AppError::BadRequest(
            "a rule needs at least one tag".to_string(),
        ));
    }
    let stored = match &input.id {
        Some(id) => Some(require_rule(&library.conn, id)?),
        None => None,
    };
    if input.is_regex
        && !keeps_stored_pattern(stored.as_ref(), input)
        && let Some(reason) = compile_regex(&input.pattern).err()
    {
        return Err(AppError::BadRequest(reason));
    }

    let now = db::now_ms();
    let tags_json = serde_json::to_string(&input.tags)
        .map_err(|error| AppError::BadRequest(format!("tags cannot be stored: {error}")))?;

    match &input.id {
        Some(id) => {
            library.conn.execute(
                "UPDATE rules SET name = ?1, pattern = ?2, is_regex = ?3, tags_json = ?4,
                                  enabled = ?5, updated_at = ?6
                 WHERE id = ?7",
                params![
                    name,
                    input.pattern,
                    input.is_regex,
                    tags_json,
                    input.enabled,
                    now,
                    id
                ],
            )?;
            require_rule(&library.conn, id)
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            library.conn.execute(
                "INSERT INTO rules (id, name, pattern, is_regex, tags_json, enabled, created_at,
                                    updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    id,
                    name,
                    input.pattern,
                    input.is_regex,
                    tags_json,
                    input.enabled,
                    now
                ],
            )?;
            require_rule(&library.conn, &id)
        }
    }
}

/// Whether this edit leaves the pattern exactly as the row already holds it,
/// kind included. An imported rule may carry a regular expression this engine
/// cannot compile — design D6 stores it rather than dropping it — and the
/// table's switch disables a rule through this same upsert, so validating a
/// pattern nobody is rewriting would leave such a rule with no off switch.
/// Marking a stored plain pattern as a regular expression *is* writing one, so
/// the kind has to match too.
fn keeps_stored_pattern(stored: Option<&Rule>, input: &RuleInput) -> bool {
    stored.is_some_and(|rule| rule.is_regex == input.is_regex && rule.pattern == input.pattern)
}

/// Delete a rule. Idempotent — a rule already gone is the same outcome as one
/// deleted now — and no image it ever tagged is touched (spec
/// `auto-tag-rules`, "Deleting").
pub fn delete(library: &Library, id: &str) -> Result<()> {
    library
        .conn
        .execute("DELETE FROM rules WHERE id = ?1", [id])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Running rules over stored images (design D9)
// ---------------------------------------------------------------------------

/// One stored image's own text, read once before the run so the walk itself
/// never holds the library across more than one image at a time.
struct StoredImage {
    id: String,
    title: Option<String>,
    adapter: Option<SiteAdapterRecord>,
    rating: Option<String>,
}

/// Apply the current rules to every non-deleted image (design D9): a run only
/// ever adds tags — no tag is removed, an image already carrying a rule's tag
/// is left unchanged by it, and a rating from a rule's `rating:` tag is set
/// only where there is none, so a hand-given rating survives. Trashed images
/// are not examined. An invalid or disabled rule never matches (design D6) and
/// is named in the report's `invalid` half rather than applied.
///
/// One transaction per image, not one for the whole run: a run over ten
/// thousand images holding one transaction would hold the library mutex and
/// the rollback journal for its whole duration, and the library is released
/// between images so a switch or a capture arriving mid-run waits for the
/// image in flight only, the same as `import::import_paths`.
pub fn run(
    library: &SharedLibrary,
    on_progress: &mut dyn FnMut(i64, i64),
) -> Result<RulesRunReport> {
    // A closed library is one refusal, not a report naming every image
    // failed — asked before anything is counted.
    with_library(library, |_| Ok(()))?;

    let (valid, mut invalid) = with_library(library, |library| read_rules(&library.conn))?;
    let images = with_library(library, |library| read_candidate_images(&library.conn))?;

    let total = images.len() as i64;
    let mut added_by: HashMap<String, i64> = HashMap::new();
    let mut examined = 0i64;
    let mut changed = 0i64;

    on_progress(0, total);
    for image in &images {
        // An image that fails stops the run, and the report of everything
        // already committed is still returned: `?` here would throw that away,
        // and the run's writes are committed per image (design D9), so the
        // caller would be told nothing about work that did land.
        //
        // FIXME: the report has no room to say the run stopped early or why —
        // the right shape is a `RulesRunReport.failed: Option<String>`, which
        // means editing the hand-mirrored `model.rs` / `packages/shared` pair
        // (Phase 1 D11) that another change holds open right now. Until it
        // lands, the reason only reaches stderr and `examined < total` is the
        // one signal a reader gets.
        let Ok(image_changed) = with_library(library, |library| {
            apply_rules_to_image(library, &valid, image, &mut added_by)
        })
        .inspect_err(|error| eprintln!("rules_run stopped at image {}: {error}", image.id)) else {
            break;
        };
        examined += 1;
        if image_changed {
            changed += 1;
        }
        on_progress(examined, total);
    }

    // `RuleRunCount.matched` carries how many images the rule *added* to, which
    // is what the spec asks a run to report; the field is named for the older
    // reading and renaming it is a `model.rs` / `packages/shared` pair edit.
    let rules_report: Vec<RuleRunCount> = valid
        .iter()
        .map(|rule| RuleRunCount {
            id: rule.id.clone(),
            name: rule.name.clone(),
            matched: *added_by.get(&rule.id).unwrap_or(&0),
            pattern_error: None,
        })
        .collect();
    invalid.sort_by_key(|entry| entry.name.to_lowercase());

    Ok(RulesRunReport {
        examined,
        changed,
        rules: rules_report,
        invalid,
    })
}

/// The rules split into the ones a run applies and the ones it can only name:
/// an invalid rule's `RuleRunCount` carries its reason and a `matched` of 0,
/// which is the `invalid` half of the report `run` builds.
fn read_rules(conn: &Connection) -> Result<(Vec<Rule>, Vec<RuleRunCount>)> {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    for entry in list(conn)? {
        match entry.pattern_error {
            Some(reason) => invalid.push(RuleRunCount {
                id: entry.rule.id,
                name: entry.rule.name,
                matched: 0,
                pattern_error: Some(reason),
            }),
            None => valid.push(entry.rule),
        }
    }
    Ok((valid, invalid))
}

fn read_candidate_images(conn: &Connection) -> Result<Vec<StoredImage>> {
    let mut stmt = conn.prepare(
        "SELECT id, page_title, adapter_json, rating FROM images WHERE deleted_at IS NULL",
    )?;
    let rows = stmt.query_map([], |row| {
        let adapter_json: Option<String> = row.get(2)?;
        Ok(StoredImage {
            id: row.get(0)?,
            title: row.get(1)?,
            adapter: adapter_json.and_then(|json| serde_json::from_str(&json).ok()),
            rating: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Match `valid` against one stored image and — through `tags.rs`'s writers,
/// one transaction for this image alone — add whatever tags it does not
/// already carry and set a rating only when it has none. Answers whether the
/// image actually changed, and tallies into `added_by` the rules that put
/// something there (see [`tally`]).
fn apply_rules_to_image(
    library: &Library,
    valid: &[Rule],
    image: &StoredImage,
    added_by: &mut HashMap<String, i64>,
) -> Result<bool> {
    let haystacks = haystacks(image.title.as_deref(), image.adapter.as_ref());
    let fired: Vec<&Rule> = valid
        .iter()
        .filter(|rule| matches(rule, &haystacks))
        .collect();
    if fired.is_empty() {
        return Ok(false);
    }

    let candidate_tags = union_tags(fired.iter().copied());
    let (candidate_tags, extracted_rating) = tags::split_rating(&candidate_tags);

    let tx = library.conn.unchecked_transaction()?;
    let existing = existing_tag_names(&tx, &image.id)?;
    let written_tags: Vec<String> = candidate_tags
        .into_iter()
        .filter(|tag| !existing.contains(tag))
        .collect();
    // A rating a rule names is written only where the image has none, so a
    // hand-given rating survives a run (spec: "A rating already given").
    let written_rating = extracted_rating.filter(|_| image.rating.is_none());
    if written_tags.is_empty() && written_rating.is_none() {
        return Ok(false);
    }

    if !written_tags.is_empty() {
        tags::add_tags(&tx, &image.id, &written_tags)?;
    }
    tags::stamp(&tx, &image.id, written_rating.as_deref())?;
    tx.commit()?;

    tally(&fired, &written_tags, written_rating.as_deref(), added_by);
    Ok(true)
}

/// Count a rule against this image only when it actually put something there —
/// one of the tags written, or the rating that was set. The spec asks the
/// report for "how many images it added tags to", not how many it matched: a
/// rule matching an image that already carries its tags added nothing, and
/// counting that match is what would make a second run report full per-rule
/// numbers beside `changed: 0`.
fn tally(
    fired: &[&Rule],
    written_tags: &[String],
    written_rating: Option<&str>,
    added_by: &mut HashMap<String, i64>,
) {
    for rule in fired {
        let (tags, rating) = tags::split_rating(&rule.tags);
        let contributed = tags.iter().any(|tag| written_tags.contains(tag))
            || (written_rating.is_some() && rating.as_deref() == written_rating);
        if contributed {
            *added_by.entry(rule.id.clone()).or_insert(0) += 1;
        }
    }
}

fn existing_tag_names(conn: &Connection, image_id: &str) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(
        "SELECT tags.name FROM image_tags
         JOIN tags ON tags.id = image_tags.tag_id
         WHERE image_tags.image_id = ?1",
    )?;
    let names = stmt.query_map([image_id], |row| row.get::<_, String>(0))?;
    Ok(names.collect::<rusqlite::Result<_>>()?)
}

// ---------------------------------------------------------------------------
// Moving rules between libraries as JSON (design D10)
// ---------------------------------------------------------------------------

/// The legacy extension's `TagRule` shape: six fields, camelCase. Kept as the
/// export and import format so a file exported by either app reads in the
/// other, and so `legacy-bundle-import` can hand the bundle's `tagRules` array
/// straight to [`import_json`] instead of writing a second importer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleJson {
    #[serde(default)]
    id: Option<String>,
    name: String,
    pattern: String,
    is_regex: bool,
    tags: Vec<String>,
    /// Present in every export this app writes; a file that omits it (the
    /// legacy's own bundles never did, but a hand-edited file might) is read
    /// as enabled, matching what a freshly created rule defaults to.
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl From<Rule> for RuleJson {
    fn from(rule: Rule) -> Self {
        RuleJson {
            id: Some(rule.id),
            name: rule.name,
            pattern: rule.pattern,
            is_regex: rule.is_regex,
            tags: rule.tags,
            enabled: rule.enabled,
        }
    }
}

/// The equivalence class two rules are the same by (design D10): name,
/// pattern, kind and the tags **sorted**, joined on a character that cannot
/// occur in a tag rather than serialised as JSON — the fingerprint is only
/// ever compared against fingerprints this same function computed, so there is
/// nothing to gain from coupling it to `serde_json`'s field order. `enabled`
/// is deliberately not part of it: a rule already present is a duplicate
/// whether or not it is switched on.
fn fingerprint(name: &str, pattern: &str, is_regex: bool, tags: &[String]) -> String {
    let mut sorted = tags.to_vec();
    sorted.sort();
    format!(
        "{name}\u{1f}{pattern}\u{1f}{is_regex}\u{1f}{}",
        sorted.join("\u{1f}")
    )
}

/// The library's rules as the legacy's `TagRule[]` JSON, pretty-printed.
/// `created_at`/`updated_at` are not exported: they are this library's own
/// bookkeeping, and an imported rule is new wherever it lands.
pub fn export_json(library: &Library) -> Result<String> {
    let rules: Vec<RuleJson> = list(&library.conn)?
        .into_iter()
        .map(|entry| RuleJson::from(entry.rule))
        .collect();
    serde_json::to_string_pretty(&rules)
        .map_err(|error| AppError::BadRequest(format!("rules cannot be exported: {error}")))
}

/// Import rules from `text` in the legacy shape (design D10). Every rule gets
/// a fresh id in this library — the file's ids belong to another one and could
/// collide — and one whose fingerprint matches a rule already here is skipped.
/// Unlike [`upsert`], an entry with a pattern the engine cannot use is stored
/// and left for the list to mark invalid rather than dropped: the file came
/// from elsewhere, in the limit the legacy extension, and refusing it would
/// lose the rule silently. A value that does not parse as a list of rule
/// objects is refused whole, with a reason, leaving the library untouched.
pub fn import_json(library: &Library, text: &str) -> Result<RulesImportReport> {
    let entries: Vec<RuleJson> = serde_json::from_str(text)
        .map_err(|error| AppError::BadRequest(format!("not a list of rules: {error}")))?;

    // One transaction for the whole file: a failure part-way through would
    // otherwise leave half a file imported, and D10's refusal of a file that is
    // not a list of rules already promises "leaving the library untouched".
    let tx = library.conn.unchecked_transaction()?;
    let mut seen: HashSet<String> = list(&tx)?
        .into_iter()
        .map(|entry| {
            fingerprint(
                &entry.rule.name,
                &entry.rule.pattern,
                entry.rule.is_regex,
                &entry.rule.tags,
            )
        })
        .collect();

    let mut report = RulesImportReport::default();
    let now = db::now_ms();
    for entry in entries {
        let key = fingerprint(&entry.name, &entry.pattern, entry.is_regex, &entry.tags);
        if !seen.insert(key) {
            report.skipped += 1;
            continue;
        }
        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|error| AppError::BadRequest(format!("tags cannot be stored: {error}")))?;
        tx.execute(
            "INSERT INTO rules (id, name, pattern, is_regex, tags_json, enabled, created_at,
                                updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                uuid::Uuid::new_v4().to_string(),
                entry.name,
                entry.pattern,
                entry.is_regex,
                tags_json,
                entry.enabled,
                now
            ],
        )?;
        report.imported += 1;
    }
    tx.commit()?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{self, IngestInput, store_image};
    use crate::model::ImageSource;

    fn rule(id: &str, pattern: &str, is_regex: bool, tags: &[&str], enabled: bool) -> Rule {
        Rule {
            id: id.to_string(),
            name: id.to_string(),
            pattern: pattern.to_string(),
            is_regex,
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            enabled,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn strs(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    // ---- matches / auto_tags (task 1.3) ----

    #[test]
    fn a_disabled_rule_never_matches() {
        let disabled = rule("r", "cat", false, &["cat"], false);
        assert!(!matches(&disabled, &strs(&["a cat page"])));
    }

    #[test]
    fn an_empty_pattern_matches_everything() {
        let matches_all = rule("r", "", false, &["all"], true);
        assert!(matches(&matches_all, &strs(&["anything at all"])));
        assert!(matches(&matches_all, &[]));
    }

    #[test]
    fn substring_matching_ignores_case_both_ways() {
        let lower = rule("r", "pixiv", false, &["pixiv"], true);
        assert!(matches(&lower, &strs(&["a PIXIV post"])));

        let upper = rule("r", "PIXIV", false, &["pixiv"], true);
        assert!(matches(&upper, &strs(&["a pixiv post"])));
    }

    #[test]
    fn regex_matching_ignores_case() {
        let bracketed = rule("r", r"^\[.+\]", true, &["doujin"], true);
        assert!(matches(&bracketed, &strs(&["[SOMEONE] a title"])));
    }

    #[test]
    fn an_unusable_regex_reports_no_match_with_the_reason_and_never_panics() {
        let broken = rule("r", "(unterminated", true, &["x"], true);
        assert!(!matches(&broken, &strs(&["(unterminated group"])));
        assert!(pattern_error(&broken).is_some());
    }

    #[test]
    fn a_match_against_the_second_haystack_when_the_first_does_not_match() {
        let handle = rule("r", "alice", false, &["fanart"], true);
        assert!(matches(&handle, &strs(&["unrelated title", "by alice"])));
    }

    #[test]
    fn an_empty_haystack_list_does_not_match_a_nonempty_pattern() {
        let named = rule("r", "cat", false, &["cat"], true);
        assert!(!matches(&named, &[]));
    }

    #[test]
    fn two_rules_naming_the_same_tag_yield_it_once() {
        let a = rule("a", "cat", false, &["fanart"], true);
        let b = rule("b", "", false, &["fanart", "extra"], true);

        let tags = auto_tags(&[a, b], &strs(&["a cat page"]));

        assert_eq!(tags, vec!["fanart".to_string(), "extra".to_string()]);
    }

    // ---- the store (task 1.4) ----

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn new_rule(name: &str, pattern: &str, is_regex: bool, tags: &[&str]) -> RuleInput {
        RuleInput {
            id: None,
            name: name.to_string(),
            pattern: pattern.to_string(),
            is_regex,
            tags: strs(tags),
            enabled: true,
        }
    }

    #[test]
    fn upsert_round_trips_a_rule() {
        let (_dir, library) = library();

        let created = upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();

        let listed = list(&library.conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].rule, created);
        assert_eq!(created.tags, vec!["pixiv".to_string()]);
    }

    #[test]
    fn editing_keeps_the_id_and_moves_updated_at() {
        let (_dir, library) = library();
        let created = upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();

        let edited = upsert(
            &library,
            &RuleInput {
                id: Some(created.id.clone()),
                name: "pixiv".to_string(),
                pattern: "pixiv.net".to_string(),
                is_regex: false,
                tags: strs(&["pixiv"]),
                enabled: true,
            },
        )
        .unwrap();

        assert_eq!(edited.id, created.id);
        assert_eq!(edited.pattern, "pixiv.net");
        assert!(edited.updated_at >= created.updated_at);
        assert_eq!(list(&library.conn).unwrap().len(), 1);
    }

    #[test]
    fn a_rule_with_an_unusable_regex_is_listed_with_its_error() {
        let (_dir, library) = library();
        library
            .conn
            .execute(
                "INSERT INTO rules (id, name, pattern, is_regex, tags_json, enabled, created_at,
                                    updated_at)
                 VALUES ('r-1', 'broken', '(unterminated', 1, '[\"x\"]', 1, 0, 0)",
                [],
            )
            .unwrap();

        let listed = list(&library.conn).unwrap();

        assert_eq!(listed.len(), 1);
        assert!(listed[0].pattern_error.is_some());
        assert!(listed[0].rule.enabled);
    }

    #[test]
    fn upsert_refuses_an_unusable_regex() {
        let (_dir, library) = library();

        let error =
            upsert(&library, &new_rule("broken", "(unterminated", true, &["x"])).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert!(list(&library.conn).unwrap().is_empty());
    }

    /// The row `import_json` is allowed to store and `upsert` may not write:
    /// a regular expression this engine cannot compile (design D6).
    fn insert_broken_rule(library: &Library) -> Rule {
        library
            .conn
            .execute(
                "INSERT INTO rules (id, name, pattern, is_regex, tags_json, enabled, created_at,
                                    updated_at)
                 VALUES ('r-1', 'broken', '(unterminated', 1, '[\"x\"]', 1, 0, 0)",
                [],
            )
            .unwrap();
        require_rule(&library.conn, "r-1").unwrap()
    }

    #[test]
    fn an_imported_rule_with_an_unusable_regex_can_still_be_disabled() {
        let (_dir, library) = library();
        let stored = insert_broken_rule(&library);

        let disabled = upsert(
            &library,
            &RuleInput {
                id: Some(stored.id.clone()),
                name: stored.name,
                pattern: stored.pattern,
                is_regex: true,
                tags: stored.tags,
                enabled: false,
            },
        )
        .unwrap();

        assert!(!disabled.enabled);
    }

    #[test]
    fn rewriting_an_unusable_regex_into_another_one_is_still_refused() {
        let (_dir, library) = library();
        let stored = insert_broken_rule(&library);

        let error = upsert(
            &library,
            &RuleInput {
                id: Some(stored.id.clone()),
                name: stored.name,
                pattern: "(still broken".to_string(),
                is_regex: true,
                tags: stored.tags,
                enabled: true,
            },
        )
        .unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert_eq!(
            require_rule(&library.conn, "r-1").unwrap().pattern,
            "(unterminated"
        );
    }

    #[test]
    fn upsert_refuses_an_empty_name_and_an_empty_tag_list() {
        let (_dir, library) = library();

        assert!(matches!(
            upsert(&library, &new_rule("", "cat", false, &["cat"])).unwrap_err(),
            AppError::BadRequest(_)
        ));
        assert!(matches!(
            upsert(&library, &new_rule("cat", "cat", false, &[])).unwrap_err(),
            AppError::BadRequest(_)
        ));
        assert!(list(&library.conn).unwrap().is_empty());
    }

    #[test]
    fn deleting_a_rule_leaves_image_tags_untouched() {
        let (_dir, library) = library();
        let created = upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        store_captured(&library, "img-1", Some("a pixiv page"), &[]);
        crate::tags::update_tags(&library, "img-1", &strs(&["pixiv"])).unwrap();

        delete(&library, &created.id).unwrap();

        assert!(list(&library.conn).unwrap().is_empty());
        assert_eq!(
            ingest::require_record(&library.conn, "img-1").unwrap().tags,
            vec!["pixiv".to_string()],
        );
    }

    #[test]
    fn enabled_rules_omits_the_disabled_ones() {
        let (_dir, library) = library();
        upsert(&library, &new_rule("on", "a", false, &["a"])).unwrap();
        upsert(
            &library,
            &RuleInput {
                enabled: false,
                ..new_rule("off", "b", false, &["b"])
            },
        )
        .unwrap();

        let enabled = enabled_rules(&library.conn).unwrap();

        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "on");
    }

    // ---- haystacks (task 2.3) ----

    fn adapter(site: &str, fields: serde_json::Value) -> SiteAdapterRecord {
        SiteAdapterRecord {
            site: site.to_string(),
            fields,
        }
    }

    #[test]
    fn an_x_record_offers_the_handle_the_post_text_and_the_original_url() {
        let record = adapter(
            "x",
            serde_json::json!({
                "handle": "alice",
                "postText": "look at this",
                "originalUrl": "https://pbs.example/media/1.jpg",
            }),
        );

        let seen = haystacks(Some("a page"), Some(&record));

        for expected in [
            "a page",
            "alice",
            "look at this",
            "https://pbs.example/media/1.jpg",
        ] {
            assert!(seen.contains(&expected.to_string()), "{seen:?}");
        }
        assert_eq!(seen.len(), 5, "the site itself is also matched: {seen:?}");
    }

    #[test]
    fn a_pixiv_record_offers_the_artist_the_work_id_and_the_title() {
        let record = adapter(
            "pixiv",
            serde_json::json!({
                "artist": "someone",
                "workId": "12345",
                "title": "a title",
                "originalUrl": "https://i.pximg.net/img/1.png",
            }),
        );

        let seen = haystacks(Some("a page"), Some(&record));

        for expected in [
            "someone",
            "12345",
            "a title",
            "https://i.pximg.net/img/1.png",
            "pixiv",
        ] {
            assert!(seen.contains(&expected.to_string()), "{seen:?}");
        }
    }

    #[test]
    fn an_unknown_field_is_matchable_too() {
        let record = adapter(
            "somewhere-new",
            serde_json::json!({ "whatIsThis": "a value" }),
        );

        let seen = haystacks(None, Some(&record));

        assert!(seen.contains(&"a value".to_string()));
    }

    #[test]
    fn no_adapter_record_leaves_only_the_title() {
        assert_eq!(
            haystacks(Some("just a title"), None),
            vec!["just a title".to_string()]
        );
    }

    #[test]
    fn the_page_and_image_addresses_are_never_in_the_list() {
        // `haystacks` never takes them as arguments at all: `store_image` and
        // `rules::run` pass only the title and the adapter record, never
        // `page_url`/`image_url` — this is the guarantee that decision rests on.
        let record = adapter("x", serde_json::json!({ "handle": "alice" }));
        let seen = haystacks(Some("a title"), Some(&record));
        assert!(!seen.iter().any(|text| text.contains("://")));
    }

    // ---- import/export (task 4.1) ----

    fn store_captured(library: &Library, id: &str, title: Option<&str>, tags: &[&str]) {
        let bytes = png_bytes();
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        store_image(
            library,
            IngestInput {
                id,
                bytes: &bytes,
                source: ImageSource::Extension,
                source_ref: None,
                image_url: None,
                page_url: None,
                page_title: title,
                adapter: None,
                rating: None,
                tags: &tags,
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn a_round_trip_through_an_empty_library_holds_the_same_rules_with_new_identities() {
        let (_dir, source) = library();
        let created = upsert(&source, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();

        let json = export_json(&source).unwrap();
        let (_dir2, target) = library();
        let report = import_json(&target, &json).unwrap();

        assert_eq!(
            report,
            RulesImportReport {
                imported: 1,
                skipped: 0
            }
        );
        let imported = &list(&target.conn).unwrap()[0].rule;
        assert_ne!(imported.id, created.id);
        assert_eq!(imported.name, created.name);
        assert_eq!(imported.tags, created.tags);
    }

    #[test]
    fn importing_the_same_text_twice_reports_every_rule_skipped_the_second_time() {
        let (_dir, source) = library();
        upsert(&source, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        let json = export_json(&source).unwrap();
        let (_dir2, target) = library();

        import_json(&target, &json).unwrap();
        let second = import_json(&target, &json).unwrap();

        assert_eq!(
            second,
            RulesImportReport {
                imported: 0,
                skipped: 1
            }
        );
        assert_eq!(list(&target.conn).unwrap().len(), 1);
    }

    #[test]
    fn a_rule_differing_only_in_one_tag_is_imported_as_a_second_rule() {
        let (_dir, library) = library();
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        let json = serde_json::json!([{
            "id": "other",
            "name": "pixiv",
            "pattern": "pixiv",
            "isRegex": false,
            "tags": ["pixiv", "extra"],
            "enabled": true,
        }])
        .to_string();

        let report = import_json(&library, &json).unwrap();

        assert_eq!(
            report,
            RulesImportReport {
                imported: 1,
                skipped: 0
            }
        );
        assert_eq!(list(&library.conn).unwrap().len(), 2);
    }

    #[test]
    fn the_same_tags_in_a_different_order_are_skipped() {
        let (_dir, library) = library();
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["a", "b"])).unwrap();
        let json = serde_json::json!([{
            "id": "other",
            "name": "pixiv",
            "pattern": "pixiv",
            "isRegex": false,
            "tags": ["b", "a"],
            "enabled": true,
        }])
        .to_string();

        let report = import_json(&library, &json).unwrap();

        assert_eq!(
            report,
            RulesImportReport {
                imported: 0,
                skipped: 1
            }
        );
    }

    #[test]
    fn a_disabled_rule_stays_disabled_on_import() {
        let (_dir, library) = library();
        let json = serde_json::json!([{
            "id": "other",
            "name": "off",
            "pattern": "x",
            "isRegex": false,
            "tags": ["x"],
            "enabled": false,
        }])
        .to_string();

        import_json(&library, &json).unwrap();

        assert!(!list(&library.conn).unwrap()[0].rule.enabled);
    }

    #[test]
    fn a_file_that_is_not_an_array_is_refused_with_the_library_unchanged() {
        let (_dir, library) = library();
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();

        let error = import_json(&library, "{\"not\": \"a list\"}").unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert_eq!(list(&library.conn).unwrap().len(), 1);
    }

    #[test]
    fn a_file_whose_second_entry_is_not_a_rule_imports_nothing() {
        let (_dir, library) = library();
        let json = serde_json::json!([
            { "name": "first", "pattern": "a", "isRegex": false, "tags": ["a"] },
            { "pattern": "b", "isRegex": false, "tags": ["b"] },
        ])
        .to_string();

        let error = import_json(&library, &json).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert!(
            list(&library.conn).unwrap().is_empty(),
            "a file is imported whole or not at all: the first entry must not survive the second"
        );
    }

    #[test]
    fn an_entry_with_an_unusable_regex_is_imported_and_listed_invalid() {
        let (_dir, library) = library();
        let json = serde_json::json!([{
            "id": "other",
            "name": "broken",
            "pattern": "(unterminated",
            "isRegex": true,
            "tags": ["x"],
            "enabled": true,
        }])
        .to_string();

        let report = import_json(&library, &json).unwrap();

        assert_eq!(report.imported, 1);
        assert!(list(&library.conn).unwrap()[0].pattern_error.is_some());
    }

    // ---- run (task 3.1) ----

    fn shared(library: Library) -> SharedLibrary {
        std::sync::Arc::new(std::sync::Mutex::new(Some(library)))
    }

    fn run_now(library: &SharedLibrary) -> RulesRunReport {
        let mut ticks = Vec::new();
        let report = run(library, &mut |done, total| ticks.push((done, total))).unwrap();
        assert_eq!(
            ticks.last().copied(),
            Some((report.examined, report.examined))
        );
        report
    }

    #[test]
    fn a_run_tags_exactly_the_matching_images_and_reports_the_counts() {
        let (_dir, library) = library();
        // Stored before the rule exists, so ingest never tags them itself
        // (design D7) — only `run` is under test here.
        store_captured(&library, "a", Some("a pixiv piece"), &[]);
        store_captured(&library, "b", Some("something else"), &[]);
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        let shared = shared(library);

        let report = run_now(&shared);

        assert_eq!(report.examined, 2);
        assert_eq!(report.changed, 1);
        assert_eq!(report.rules.len(), 1);
        assert_eq!(report.rules[0].name, "pixiv");
        assert_eq!(report.rules[0].matched, 1);
        assert_eq!(report.rules[0].pattern_error, None);
        with_library(&shared, |library| {
            assert_eq!(
                ingest::require_record(&library.conn, "a").unwrap().tags,
                vec!["pixiv".to_string()],
            );
            assert!(
                ingest::require_record(&library.conn, "b")
                    .unwrap()
                    .tags
                    .is_empty()
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn a_second_run_changes_nothing() {
        let (_dir, library) = library();
        store_captured(&library, "a", Some("a pixiv piece"), &[]);
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        let shared = shared(library);
        run_now(&shared);

        let second = run_now(&shared);

        assert_eq!(second.changed, 0);
        // The per-rule count is what the rule added, not what it matched: the
        // rule still matches this image, and adding nothing has to read as 0.
        assert_eq!(second.rules[0].matched, 0);
    }

    #[test]
    fn no_existing_tag_is_removed() {
        let (_dir, library) = library();
        store_captured(&library, "a", Some("a pixiv piece"), &["keep-me"]);
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        let shared = shared(library);

        run_now(&shared);

        with_library(&shared, |library| {
            let tags = ingest::require_record(&library.conn, "a").unwrap().tags;
            assert!(tags.contains(&"keep-me".to_string()));
            assert!(tags.contains(&"pixiv".to_string()));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn a_hand_given_rating_survives_while_an_unrated_image_gets_the_rules() {
        let (_dir, library) = library();
        store_captured(&library, "rated", Some("x"), &[]);
        store_captured(&library, "unrated", Some("y"), &[]);
        crate::tags::set_rating(&library, "rated", Some("s")).unwrap();
        upsert(&library, &new_rule("explicit", "", false, &["rating:e"])).unwrap();
        let shared = shared(library);

        run_now(&shared);

        with_library(&shared, |library| {
            assert_eq!(
                ingest::require_record(&library.conn, "rated")
                    .unwrap()
                    .rating
                    .as_deref(),
                Some("s"),
            );
            assert_eq!(
                ingest::require_record(&library.conn, "unrated")
                    .unwrap()
                    .rating
                    .as_deref(),
                Some("e"),
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn trashed_images_are_skipped() {
        let (_dir, library) = library();
        store_captured(&library, "a", Some("a pixiv piece"), &[]);
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        crate::query::mark_deleted(&library.conn, "a", 1);
        let shared = shared(library);

        let report = run_now(&shared);

        assert_eq!(report.examined, 0);
        assert_eq!(report.changed, 0);
    }

    #[test]
    fn an_invalid_rule_is_named_while_the_others_still_apply() {
        let (_dir, library) = library();
        store_captured(&library, "a", Some("a pixiv piece"), &[]);
        upsert(&library, &new_rule("pixiv", "pixiv", false, &["pixiv"])).unwrap();
        library
            .conn
            .execute(
                "INSERT INTO rules (id, name, pattern, is_regex, tags_json, enabled, created_at,
                                    updated_at)
                 VALUES ('broken', 'broken', '(unterminated', 1, '[\"x\"]', 1, 0, 0)",
                [],
            )
            .unwrap();
        let shared = shared(library);

        let report = run_now(&shared);

        assert_eq!(report.changed, 1);
        assert_eq!(report.invalid.len(), 1);
        assert_eq!(report.invalid[0].name, "broken");
        assert!(report.invalid[0].pattern_error.is_some());
    }

    #[test]
    fn a_run_with_no_library_open_is_one_refusal() {
        let closed = SharedLibrary::default();

        let mut ticks = 0;
        let error = run(&closed, &mut |_, _| ticks += 1).unwrap_err();

        assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        assert_eq!(ticks, 0);
    }
}

//! Posting one image to a configured Danbooru-compatible booru
//! (`booru-upload`): the site list and its host-slug ids (`sites`), the OS
//! credential store behind a trait so no test touches a real keychain
//! (`credentials`), the four-call HTTP sequence over a plain `reqwest` client
//! (`client`), the orchestration that turns those calls into a
//! `BooruUploadOutcome` (`upload`), and the single durable write once a post
//! exists (`posts`).
//!
//! `commands.rs` is the only caller of `upload::run` and `posts::record`; it
//! owns the two moments the library mutex is taken (design D5), since neither
//! this module nor `client` ever sees a `Library`.

pub mod client;
pub mod credentials;
pub mod posts;
pub mod sites;
pub mod upload;

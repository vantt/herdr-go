//! Security boundary — pure functions, called everywhere, depending on nothing
//! (decision 4e3ef1a1: not a hexagonal port). herdr's socket has no auth, so the
//! gateway is the single security boundary; these validators are its teeth.
//!
//! **Honest limit** (airemote D1): this is *not* a sandbox. It governs which
//! paths the gateway hands to the runtime — once an agent runs, nothing here
//! stops it typing any command the login account can run.

pub mod paths;
pub mod redact;
pub mod slug;

pub use paths::{Boundary, PathRefusal};
pub use redact::redact;
pub use slug::{is_canonical_slug, sanitize_slug, EmptySlug};

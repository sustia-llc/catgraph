//! The escape hatch must exist in `.rs`, not only in column-0 Markdown.
//!
//! An earlier `FENCE` matched only `^\s*(```|~~~)`, so a fence behind a `///`
//! or `//!` prefix was not a fence and the documented escape hatch did not
//! exist in one of the two suffixes this guard was written for.
//!
//! ```text
//! **9 999**<!--m:fixture.plain--> is WRONG and must be skipped
//! ```
//!
//! Outside the fence, a correct citation: **7 828**<!--m:fixture.plain-->.

/// Indented fences behind `///` must work too.
///
/// ```
/// **9 999**<!--m:fixture.grouped--> is WRONG and must be skipped
/// ```
pub fn documented() {}

use crate::dcc::types::VersionTruth;

/// The single source of truth for runtime version.
/// Reads from Cargo.toml at compile time via env!().
pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Evaluate version truth.
/// - runtime_version: the compiled-in Cargo version
/// - ui_version: the version string displayed in the UI
///
/// Returns Ok if they match, Conflict if they differ, Undefined if either is empty.
pub fn evaluate_version_truth(runtime_version: &str, ui_version: &str) -> VersionTruth {
    if runtime_version.is_empty() || ui_version.is_empty() {
        return VersionTruth::Undefined;
    }
    if runtime_version != ui_version {
        return VersionTruth::Conflict;
    }
    VersionTruth::Ok
}

//! Provisional branding, centralized so the final name/binary can change.
//!
//! ROADMAP decision: binary name is NOT final. Keep every user-visible
//! string derived from these constants. See ADR 0006.

/// Internal working binary name. Final branding TBD before public launch.
pub const BINARY_NAME: &str = "aicc";

/// Project config filename (TOML, versioned). See ADR 0005.
pub const CONFIG_FILENAME: &str = ".context.toml";

/// Display name used in docs/CLI until final branding is chosen.
pub const PRODUCT_DISPLAY_NAME: &str = "AI Codebase Context (provisional name)";

/// One-line tagline (positioning, not a feature claim).
pub const PRODUCT_TAGLINE: &str = "Turn any codebase into AI-ready context — locally.";

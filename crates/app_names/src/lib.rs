//! Product naming for Forge editions.
//!
//! The single source of truth is `editions.json` at the repository root. The
//! build script validates it and generates the constants included below, so
//! Rust code, packaging scripts, and CI all read the same catalog.
//!
//! Select an edition with a Cargo feature (`edition-zed`, `edition-super`,
//! `edition-lite`) or with the `FORGE_EDITION` environment variable. With
//! neither, `forge-super` is used.

/// Static metadata describing one Forge edition.
#[derive(Debug, Clone, Copy)]
pub struct ProductInfo {
    /// Catalog key, e.g. `forge-zed`.
    pub key: &'static str,
    /// Brand shared by every edition, e.g. `Forge`.
    pub brand: &'static str,
    /// Edition qualifier, e.g. `Zed Edition`, `Super`, `Lite`.
    pub qualifier: &'static str,
    /// Channel-qualified display name, e.g. `Mentenaz Forge Super Dev`.
    pub display: &'static str,
    /// Channel-qualified data/config directory name (mixed case).
    pub dir: &'static str,
    /// Lowercase form of [`ProductInfo::dir`], used for XDG-style paths.
    pub dir_lower: &'static str,
    /// Installed executable / app-bundle name, e.g. `Forge Super`.
    pub install_name: &'static str,
    /// Command users type, e.g. `forge` or `forge-super`.
    pub command: &'static str,
    /// Base identifier for the single-instance mutex and named pipe.
    pub ipc: &'static str,
    /// Base reverse-DNS identifier (bundle id / app id).
    pub id: &'static str,
    /// URL scheme associated with the edition (reserved for future use).
    pub scheme: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/product.rs"));

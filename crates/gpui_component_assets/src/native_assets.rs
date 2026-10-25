use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

/// Native implementation using RustEmbed
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

impl Assets {
    /// Create a new Assets instance. The endpoint parameter is ignored for native builds.
    pub fn new(_endpoint: impl Into<SharedString>) -> Self {
        Self
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{}\"", path))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_icons_are_listed_under_the_icons_folder() {
        let icons = Assets.list("icons/").unwrap();
        assert!(!icons.is_empty());
        assert!(icons.iter().all(|path| path.starts_with("icons/")));
        assert!(icons.iter().all(|path| path.ends_with(".svg")));
    }

    #[test]
    fn every_listed_icon_loads_as_svg() {
        for path in Assets.list("icons/").unwrap() {
            let data = Assets
                .load(&path)
                .unwrap()
                .unwrap_or_else(|| panic!("{path} listed but not loadable"));
            let text = String::from_utf8_lossy(&data);
            assert!(text.contains("<svg"), "{path} is not an svg document");
        }
    }

    #[test]
    fn fork_added_icons_are_bundled() {
        // Panels in this fork look these up by `IconName`; a missing file
        // only shows as a blank icon at runtime.
        for name in ["Helm.svg", "Forge_Cockpit2.svg", "DotNet_NBG.svg"] {
            let path = format!("icons/{name}");
            assert!(Assets.load(&path).unwrap().is_some(), "{path} missing");
        }
    }

    #[test]
    fn empty_and_unknown_paths() {
        assert!(Assets.load("").unwrap().is_none());
        assert!(Assets.load("icons/definitely-not-an-icon.svg").is_err());
        assert!(Assets.list("no-such-folder/").unwrap().is_empty());
    }

    #[test]
    fn new_ignores_its_endpoint_on_native() {
        let assets = Assets::new("https://example.test/assets");
        assert!(!assets.list("icons/").unwrap().is_empty());
    }
}

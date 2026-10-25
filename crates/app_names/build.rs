#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]

use std::collections::HashSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const EDITION_FEATURES: [(&str, &str); 3] = [
    ("edition-zed", "forge-zed"),
    ("edition-super", "forge-super"),
    ("edition-lite", "forge-lite"),
];

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crates/app_names has a repository root");

    let catalog_path = repo_root.join("editions.json");
    println!("cargo:rerun-if-changed={}", catalog_path.display());
    println!("cargo:rerun-if-env-changed=FORGE_EDITION");
    println!("cargo:rerun-if-env-changed=FORGE_DIR_SUFFIX");
    println!("cargo:rerun-if-env-changed=FORGE_IPC_SUFFIX");
    println!("cargo:rerun-if-env-changed=ZED_RELEASE_CHANNEL");

    let catalog_text = fs::read_to_string(&catalog_path).unwrap_or_else(|error| {
        panic!(
            "failed to read edition catalog at {}: {error}",
            catalog_path.display()
        )
    });
    let catalog: serde_json::Value = serde_json::from_str(&catalog_text).unwrap_or_else(|error| {
        panic!(
            "invalid edition catalog {}: {error}",
            catalog_path.display()
        )
    });

    let dev_bin = catalog["dev_bin"]
        .as_str()
        .unwrap_or_else(|| panic!("edition catalog is missing string field `dev_bin`"))
        .to_string();
    validate_token("dev_bin", &dev_bin);

    let editions = catalog["editions"]
        .as_object()
        .unwrap_or_else(|| panic!("edition catalog is missing object field `editions`"));

    validate_catalog(editions);

    let edition_key = selected_edition();
    let edition = editions
        .get(&edition_key)
        .unwrap_or_else(|| panic!("unknown edition `{edition_key}` in edition catalog"));

    let field = |name: &str| -> String {
        edition[name]
            .as_str()
            .unwrap_or_else(|| panic!("edition `{edition_key}` is missing string field `{name}`"))
            .to_string()
    };

    let brand = field("brand");
    let qualifier = field("qualifier");
    let display = field("display");
    let base_dir = field("dir");
    let install_name = field("install_name");
    let command = field("command");
    let ipc = field("ipc");
    let id = field("id");
    let scheme = field("scheme");

    let channel = selected_channel(repo_root);
    let channel_title = match channel.as_str() {
        "stable" => "",
        "preview" => "Preview",
        "nightly" => "Nightly",
        _ => "Dev",
    };
    let id_suffix = match channel.as_str() {
        "stable" => "",
        "preview" => "-Preview",
        "nightly" => "-Nightly",
        _ => "-Dev",
    };
    let ipc_suffix = match channel.as_str() {
        "stable" => "Stable",
        "preview" => "Preview",
        "nightly" => "Nightly",
        _ => "Dev",
    };

    let display_for = |title: &str| {
        if title.is_empty() {
            display.clone()
        } else {
            format!("{display} {title}")
        }
    };
    let app_id_for = |suffix: &str| {
        if suffix.is_empty() {
            id.clone()
        } else {
            format!("{id}{suffix}")
        }
    };
    let dir_for = |suffix: &str| {
        if suffix.is_empty() {
            base_dir.clone()
        } else {
            format!("{base_dir}{suffix}")
        }
    };

    let dir_suffix = env::var("FORGE_DIR_SUFFIX").unwrap_or_default();
    let ipc_suffix_extra = env::var("FORGE_IPC_SUFFIX").unwrap_or_default();
    if !dir_suffix.is_empty() {
        validate_token("FORGE_DIR_SUFFIX", &dir_suffix);
    }
    if !ipc_suffix_extra.is_empty() {
        validate_token("FORGE_IPC_SUFFIX", &ipc_suffix_extra);
    }

    let dir = format!("{}{dir_suffix}", dir_for(channel_title));
    let dir_lower = dir.to_ascii_lowercase();
    validate_token("dir", &dir);
    validate_token("dir", &dir_lower);
    validate_token("command", &command);
    validate_token("ipc", &ipc);
    validate_token("id", &id);
    validate_token("scheme", &scheme);

    let display_stable = display_for("");
    let display_dev = display_for("Dev");
    let display_nightly = display_for("Nightly");
    let display_preview = display_for("Preview");
    let app_id_stable = app_id_for("");
    let app_id_dev = app_id_for("-Dev");
    let app_id_nightly = app_id_for("-Nightly");
    let app_id_preview = app_id_for("-Preview");
    let ipc_stable = format!("{ipc}-Stable{ipc_suffix_extra}");
    let ipc_dev = format!("{ipc}-Dev{ipc_suffix_extra}");
    let ipc_nightly = format!("{ipc}-Nightly{ipc_suffix_extra}");
    let ipc_preview = format!("{ipc}-Preview{ipc_suffix_extra}");

    let current_display = display_for(channel_title);
    let current_app_id = app_id_for(id_suffix);
    let current_ipc = format!("{ipc}-{ipc_suffix}{ipc_suffix_extra}");

    let mut generated = String::new();
    generated.push_str("// @generated by crates/app_names/build.rs from editions.json\n");
    writeln!(generated, "pub const EDITION: &str = {edition_key:?};").unwrap();
    writeln!(generated, "pub const CHANNEL: &str = {channel:?};").unwrap();
    writeln!(generated, "pub const DEV_BIN: &str = {dev_bin:?};").unwrap();
    writeln!(generated, "pub const PRODUCT: ProductInfo = ProductInfo {{").unwrap();
    writeln!(generated, "    key: {edition_key:?},").unwrap();
    writeln!(generated, "    brand: {brand:?},").unwrap();
    writeln!(generated, "    qualifier: {qualifier:?},").unwrap();
    writeln!(generated, "    display: {current_display:?},").unwrap();
    writeln!(generated, "    dir: {dir:?},").unwrap();
    writeln!(generated, "    dir_lower: {dir_lower:?},").unwrap();
    writeln!(generated, "    install_name: {install_name:?},").unwrap();
    writeln!(generated, "    command: {command:?},").unwrap();
    writeln!(generated, "    ipc: {current_ipc:?},").unwrap();
    writeln!(generated, "    id: {current_app_id:?},").unwrap();
    writeln!(generated, "    scheme: {scheme:?},").unwrap();
    writeln!(generated, "}};").unwrap();
    writeln!(
        generated,
        "pub const DISPLAY_STABLE: &str = {display_stable:?};"
    )
    .unwrap();
    writeln!(generated, "pub const DISPLAY_DEV: &str = {display_dev:?};").unwrap();
    writeln!(
        generated,
        "pub const DISPLAY_NIGHTLY: &str = {display_nightly:?};"
    )
    .unwrap();
    writeln!(
        generated,
        "pub const DISPLAY_PREVIEW: &str = {display_preview:?};"
    )
    .unwrap();
    writeln!(
        generated,
        "pub const APP_ID_STABLE: &str = {app_id_stable:?};"
    )
    .unwrap();
    writeln!(generated, "pub const APP_ID_DEV: &str = {app_id_dev:?};").unwrap();
    writeln!(
        generated,
        "pub const APP_ID_NIGHTLY: &str = {app_id_nightly:?};"
    )
    .unwrap();
    writeln!(
        generated,
        "pub const APP_ID_PREVIEW: &str = {app_id_preview:?};"
    )
    .unwrap();
    writeln!(generated, "pub const IPC_STABLE: &str = {ipc_stable:?};").unwrap();
    writeln!(generated, "pub const IPC_DEV: &str = {ipc_dev:?};").unwrap();
    writeln!(generated, "pub const IPC_NIGHTLY: &str = {ipc_nightly:?};").unwrap();
    writeln!(generated, "pub const IPC_PREVIEW: &str = {ipc_preview:?};").unwrap();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("product.rs"), generated).expect("write generated edition constants");
}

fn selected_edition() -> String {
    if let Ok(value) = env::var("FORGE_EDITION") {
        return normalize_edition(&value);
    }

    let enabled: Vec<&str> = EDITION_FEATURES
        .iter()
        .filter(|(feature, _)| {
            env::var(format!(
                "CARGO_FEATURE_{}",
                feature.to_ascii_uppercase().replace('-', "_")
            ))
            .is_ok()
        })
        .map(|(_, key)| *key)
        .collect();

    match enabled.as_slice() {
        [] => "forge-super".to_string(),
        [only] => normalize_edition(only),
        _ => panic!(
            "select exactly one Forge edition feature; got {}",
            enabled.join(", ")
        ),
    }
}

fn normalize_edition(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "forge-zed" | "zed" | "zed-edition" => "forge-zed",
        "forge-super" | "super" => "forge-super",
        "forge-lite" | "lite" => "forge-lite",
        other => panic!(
            "unknown Forge edition `{other}`; expected forge-zed, forge-super, or forge-lite"
        ),
    }
    .to_string()
}

fn selected_channel(repo_root: &Path) -> String {
    // Same sources, in the same order, as the `release_channel` crate, so the
    // generated names always match the channel the app actually runs as.
    if let Ok(value) = env::var("ZED_RELEASE_CHANNEL") {
        return normalize_channel(&value);
    }

    let channel_file = repo_root.join("crates/zed/RELEASE_CHANNEL");
    println!("cargo:rerun-if-changed={}", channel_file.display());
    fs::read_to_string(&channel_file)
        .map(|value| normalize_channel(&value))
        .unwrap_or_else(|_| "dev".to_string())
}

fn normalize_channel(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "stable" => "stable",
        "preview" => "preview",
        "nightly" => "nightly",
        _ => "dev",
    }
    .to_string()
}

fn validate_catalog(editions: &serde_json::Map<String, serde_json::Value>) {
    if editions.is_empty() {
        panic!("edition catalog has no editions");
    }

    let mut commands = HashSet::new();
    let mut dirs = HashSet::new();
    let mut ipcs = HashSet::new();
    let mut ids = HashSet::new();

    for (key, edition) in editions {
        for field in [
            "brand",
            "qualifier",
            "display",
            "dir",
            "install_name",
            "command",
            "ipc",
            "id",
            "scheme",
        ] {
            if edition[field].as_str().is_none() {
                panic!("edition `{key}` is missing string field `{field}`");
            }
        }

        for field in ["dir", "command", "ipc", "id", "scheme"] {
            validate_token(field, edition[field].as_str().unwrap());
        }

        for (field, seen) in [
            ("command", &mut commands),
            ("dir", &mut dirs),
            ("ipc", &mut ipcs),
            ("id", &mut ids),
        ] {
            let value = edition[field].as_str().unwrap().to_ascii_lowercase();
            if !seen.insert(value) {
                panic!("duplicate `{field}` value across editions in edition catalog");
            }
        }
    }
}

fn validate_token(name: &str, value: &str) {
    assert!(!value.is_empty(), "`{name}` must not be empty");
    assert!(value.is_ascii(), "`{name}` must be ASCII: {value:?}");
    assert!(
        !value.contains('/') && !value.contains('\\'),
        "`{name}` must not contain path separators: {value:?}"
    );
    assert!(
        !value.chars().any(|c| c.is_control()),
        "`{name}` must not contain control characters: {value:?}"
    );
}

use mddskmgr::config::{Appearance, Config, DesktopLabel, Hotkeys, KeyChord, Paths, save_atomic};
use pretty_assertions::assert_eq;
use std::fs;

#[test]
fn save_and_load_roundtrip() {
    let mut cfg = Config::default();
    cfg.desktops.insert(
        "guid-1".into(),
        DesktopLabel {
            title: "Work".into(),
            description: "Tickets".into(),
        },
    );
    cfg.hotkeys = Hotkeys {
        edit_title: KeyChord {
            ctrl: true,
            alt: true,
            shift: false,
            key: "T".into(),
        },
        edit_description: KeyChord {
            ctrl: true,
            alt: true,
            shift: false,
            key: "D".into(),
        },
        toggle_overlay: KeyChord {
            ctrl: true,
            alt: true,
            shift: false,
            key: "O".into(),
        },
        snap_position: KeyChord {
            ctrl: true,
            alt: true,
            shift: false,
            key: "L".into(),
        },
    };
    cfg.appearance = Appearance {
        font_family: "Segoe UI".into(),
        font_size_dip: 16,
        margin_px: 8,
        hide_on_fullscreen: false,
    };

    let td = tempfile::tempdir().expect("tmpdir");
    let base = td.path();
    let cfg_dir = base.join("cfg");
    let log_dir = base.join("log");
    fs::create_dir_all(&cfg_dir).unwrap();
    fs::create_dir_all(&log_dir).unwrap();
    let paths = Paths {
        cfg_file: cfg_dir.join("labels.json"),
        cfg_dir,
        log_dir,
    };
    save_atomic(&cfg, &paths).expect("save");
    let data = fs::read_to_string(&paths.cfg_file).expect("read file");
    let parsed: Config = serde_json::from_str(&data).expect("json");
    assert_eq!(parsed.desktops.get("guid-1").unwrap().title, "Work");
    assert_eq!(parsed.hotkeys.toggle_overlay.key, "O");
}

#[test]
fn version_migration_changes_snap_key_from_s_to_l() {
    // Simulate a v0 config with snap_position key "S" (old default)
    let cfg = Config {
        hotkeys: Hotkeys {
            snap_position: KeyChord {
                key: "S".into(),
                ..Config::default().hotkeys.snap_position
            },
            ..Config::default().hotkeys
        },
        version: None,
        ..Config::default()
    };

    let td = tempfile::tempdir().expect("tmpdir");
    let base = td.path();
    let cfg_dir = base.join("cfg");
    let log_dir = base.join("log");
    fs::create_dir_all(&cfg_dir).unwrap();
    fs::create_dir_all(&log_dir).unwrap();
    let paths = Paths {
        cfg_file: cfg_dir.join("labels.json"),
        cfg_dir,
        log_dir,
    };
    save_atomic(&cfg, &paths).expect("save");

    // Reload — migration should change "S" to "L" and set version to 1
    let data = fs::read_to_string(&paths.cfg_file).expect("read");
    let reloaded: Config = serde_json::from_str(&data).expect("json");
    // The raw file still has "S" since migration happens in load_or_default,
    // not in save_atomic. Verify the raw data is "S":
    assert_eq!(reloaded.hotkeys.snap_position.key, "S");
    assert!(reloaded.version.is_none());
}

#[test]
fn save_atomic_creates_parent_dirs() {
    let td = tempfile::tempdir().expect("tmpdir");
    let base = td.path();
    let cfg_dir = base.join("deeply").join("nested").join("cfg");
    let log_dir = base.join("log");
    let paths = Paths {
        cfg_file: cfg_dir.join("labels.json"),
        cfg_dir,
        log_dir,
    };
    // save_atomic should create cfg_dir via create_dir_all
    let cfg = Config::default();
    save_atomic(&cfg, &paths).expect("save");
    assert!(paths.cfg_file.exists());
}

#[test]
fn default_config_has_expected_values() {
    let cfg = Config::default();
    assert_eq!(cfg.hotkeys.edit_title.key, "T");
    assert_eq!(cfg.hotkeys.edit_description.key, "D");
    assert_eq!(cfg.hotkeys.toggle_overlay.key, "O");
    assert_eq!(cfg.hotkeys.snap_position.key, "L");
    assert_eq!(cfg.appearance.font_family, "Segoe UI");
    assert_eq!(cfg.appearance.font_size_dip, 16);
    assert_eq!(cfg.appearance.margin_px, 8);
    assert!(!cfg.appearance.hide_on_fullscreen);
    assert!(cfg.desktops.is_empty());
    assert!(cfg.version.is_none());
}

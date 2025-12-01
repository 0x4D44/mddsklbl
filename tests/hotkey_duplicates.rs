use mddskmgr::config::{Hotkeys, KeyChord};
use mddskmgr::hotkeys::has_duplicates;

#[test]
fn detects_duplicate_hotkeys() {
    let mut hk = Hotkeys {
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
            key: "S".into(),
        },
    };
    assert!(!has_duplicates(&hk));
    // Collide description with title
    hk.edit_description.key = "t".into();
    assert!(has_duplicates(&hk));
}

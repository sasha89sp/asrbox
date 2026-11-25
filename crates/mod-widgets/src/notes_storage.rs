use std::fs;
use std::path::Path;

use asrbox_platform_api::WidgetLayer;

use crate::notes_model::{
    FolderNode, NoteDocument, NotesDb, NotesWidgetConfig, TaskItem, MAX_NOTES_HEIGHT,
    MAX_NOTES_WIDTH, MIN_NOTES_HEIGHT, MIN_NOTES_WIDTH,
};
use crate::storage::{MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT};

pub fn load_notes_config(path: &Path) -> Result<NotesWidgetConfig, String> {
    if !path.exists() {
        return Ok(NotesWidgetConfig::default());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("read notes.cfg: {e}"))?;
    Ok(parse_notes_config(&raw))
}

pub fn save_notes_config(path: &Path, cfg: &NotesWidgetConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all notes.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_notes_config(cfg)).map_err(|e| format!("write notes.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename notes.cfg: {e}"))?;
    Ok(())
}

pub fn parse_notes_config(raw: &str) -> NotesWidgetConfig {
    let mut cfg = NotesWidgetConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let val = v.trim();
        let b = val == "1" || val.eq_ignore_ascii_case("true");
        match k.trim() {
            "visible" => cfg.visible = b,
            "x" => {
                if let Ok(n) = val.parse() {
                    cfg.x = n;
                }
            }
            "y" => {
                if let Ok(n) = val.parse() {
                    cfg.y = n;
                }
            }
            "width" => {
                if let Ok(n) = val.parse::<u32>() {
                    cfg.width = n.clamp(MIN_NOTES_WIDTH, MAX_NOTES_WIDTH);
                }
            }
            "height" => {
                if let Ok(n) = val.parse::<u32>() {
                    cfg.height = n.clamp(MIN_NOTES_HEIGHT, MAX_NOTES_HEIGHT);
                }
            }
            "layer" => cfg.layer = parse_layer(val),
            "opacity" => {
                if let Ok(n) = val.parse::<u8>() {
                    cfg.opacity_percent = n.clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
                }
            }
            "show_folders_pane" => cfg.show_folders_pane = b,
            "show_list_pane" => cfg.show_list_pane = b,
            "show_editor_pane" => cfg.show_editor_pane = b,
            "active_folder_id" => {
                if let Ok(n) = val.parse() {
                    cfg.active_folder_id = n;
                }
            }
            "active_note_id" => {
                if let Ok(n) = val.parse() {
                    cfg.active_note_id = n;
                }
            }
            _ => {}
        }
    }
    cfg.ensure_at_least_one_pane();
    cfg
}

pub fn serialize_notes_config(cfg: &NotesWidgetConfig) -> String {
    let layer = match cfg.layer {
        WidgetLayer::Wallpaper => "wallpaper",
        WidgetLayer::Normal => "normal",
        WidgetLayer::Topmost => "topmost",
    };
    let f = |b: bool| if b { 1 } else { 0 };
    format!(
        "# Настройки виджета Заметок и задач (asrbox)\n\
         settings_version=1\n\
         visible={}\n\
         x={}\n\
         y={}\n\
         width={}\n\
         height={}\n\
         layer={layer}\n\
         opacity={}\n\
         show_folders_pane={}\n\
         show_list_pane={}\n\
         show_editor_pane={}\n\
         active_folder_id={}\n\
         active_note_id={}\n",
        f(cfg.visible),
        cfg.x,
        cfg.y,
        cfg.width,
        cfg.height,
        cfg.opacity_percent,
        f(cfg.show_folders_pane),
        f(cfg.show_list_pane),
        f(cfg.show_editor_pane),
        cfg.active_folder_id,
        cfg.active_note_id,
    )
}

pub fn load_notes_db(path: &Path) -> NotesDb {
    let Ok(raw) = fs::read_to_string(path) else {
        return NotesDb::default_seeded();
    };
    let db = parse_notes_db(&raw);
    if db.folders.is_empty() {
        NotesDb::default_seeded()
    } else {
        db
    }
}

pub fn save_notes_db(path: &Path, db: &NotesDb) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all notes_db: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_notes_db(db)).map_err(|e| format!("write notes_db: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename notes_db: {e}"))?;
    Ok(())
}

pub fn serialize_notes_db(db: &NotesDb) -> String {
    let mut out = String::new();
    out.push_str("# Локальная база заметок и задач asrbox\n");
    out.push_str(&format!("version=1\nnext_id={}\n", db.next_id));
    for folder in &db.folders {
        let parent = folder.parent_id.map(|p| p.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "folder={}|{}|{}|{}\n",
            folder.id,
            parent,
            if folder.expanded { 1 } else { 0 },
            escape_text(&folder.name)
        ));
    }
    for note in &db.notes {
        out.push_str("[note]\n");
        out.push_str(&format!("id={}\n", note.id));
        out.push_str(&format!("folder_id={}\n", note.folder_id));
        out.push_str(&format!("pinned={}\n", if note.pinned { 1 } else { 0 }));
        out.push_str(&format!("updated_at={}\n", escape_text(&note.updated_at)));
        out.push_str(&format!("title={}\n", escape_text(&note.title)));
        out.push_str(&format!("body={}\n", escape_text(&note.body)));
        for task in &note.tasks {
            out.push_str(&format!(
                "task={}|{}|{}\n",
                task.id,
                if task.done { 1 } else { 0 },
                escape_text(&task.text)
            ));
        }
    }
    out
}

pub fn parse_notes_db(raw: &str) -> NotesDb {
    let mut db = NotesDb {
        next_id: 100,
        folders: Vec::new(),
        notes: Vec::new(),
    };
    let mut cur_note: Option<NoteDocument> = None;

    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "[note]" {
            if let Some(n) = cur_note.take() {
                db.notes.push(n);
            }
            cur_note = Some(NoteDocument {
                id: 0,
                folder_id: 1,
                title: String::new(),
                body: String::new(),
                tasks: Vec::new(),
                updated_at: String::new(),
                pinned: false,
            });
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        if let Some(note) = cur_note.as_mut() {
            match k.trim() {
                "id" => {
                    if let Ok(n) = v.trim().parse() {
                        note.id = n;
                    }
                }
                "folder_id" => {
                    if let Ok(n) = v.trim().parse() {
                        note.folder_id = n;
                    }
                }
                "pinned" => note.pinned = v.trim() == "1",
                "updated_at" => note.updated_at = unescape_text(v),
                "title" => note.title = unescape_text(v),
                "body" => note.body = unescape_text(v),
                "task" => {
                    let mut p = v.splitn(3, '|');
                    if let (Some(id_s), Some(done_s), Some(txt)) = (p.next(), p.next(), p.next()) {
                        if let Ok(tid) = id_s.trim().parse::<u64>() {
                            note.tasks.push(TaskItem {
                                id: tid,
                                done: done_s.trim() == "1",
                                text: unescape_text(txt),
                            });
                        }
                    }
                }
                _ => {}
            }
        } else {
            match k.trim() {
                "next_id" => {
                    if let Ok(n) = v.trim().parse() {
                        db.next_id = n;
                    }
                }
                "folder" => {
                    let mut p = v.splitn(4, '|');
                    if let (Some(id_s), Some(par_s), Some(exp_s), Some(name)) =
                        (p.next(), p.next(), p.next(), p.next())
                    {
                        if let Ok(fid) = id_s.trim().parse::<u64>() {
                            db.folders.push(FolderNode {
                                id: fid,
                                parent_id: par_s.trim().parse::<u64>().ok(),
                                expanded: exp_s.trim() == "1",
                                name: unescape_text(name),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(n) = cur_note {
        db.notes.push(n);
    }
    db
}

fn parse_layer(val: &str) -> WidgetLayer {
    match val {
        "wallpaper" => WidgetLayer::Wallpaper,
        "topmost" => WidgetLayer::Topmost,
        _ => WidgetLayer::Normal,
    }
}

fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '|' => out.push_str("\\p"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('p') => out.push('|'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_config_and_db_roundtrip() {
        let cfg = NotesWidgetConfig {
            visible: true,
            show_folders_pane: false,
            ..NotesWidgetConfig::default()
        };
        assert_eq!(parse_notes_config(&serialize_notes_config(&cfg)), cfg);

        let db = NotesDb::default_seeded();
        let parsed = parse_notes_db(&serialize_notes_db(&db));
        assert_eq!(parsed, db);
    }
}

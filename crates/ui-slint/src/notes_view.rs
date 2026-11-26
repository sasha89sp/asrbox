use std::cell::RefCell;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use asrbox_mod_about::AppPaths;
use asrbox_mod_widgets::{save_notes_config, save_notes_db, NotesDb, NotesWidgetConfig};
use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::generated::ui::{NotesWindow, SlintFolderRow, SlintNoteCardRow, SlintTaskRow};
use crate::ipc_proto::encode_ui_cmd;

pub struct LocalNotesState {
    pub paths: AppPaths,
    pub db_path: PathBuf,
    pub cfg: NotesWidgetConfig,
    pub db: NotesDb,
    pub search: String,
    pub dirty: bool,
    pub last_dark: bool,
    pub autosave_timer: Timer,
}

impl LocalNotesState {
    pub fn ensure_selection(&mut self) {
        if !self.db.folders.iter().any(|f| f.id == self.cfg.active_folder_id) {
            self.cfg.active_folder_id = self.db.folders.first().map_or(1, |f| f.id);
        }
        let visible = self.db.filtered_notes(self.cfg.active_folder_id, &self.search);
        if !visible.iter().any(|n| n.id == self.cfg.active_note_id) {
            self.cfg.active_note_id = visible.first().map_or(0, |n| n.id);
        }
    }

    pub fn persist_db(&mut self) {
        let _ = save_notes_db(&self.db_path, &self.db);
        self.dirty = false;
    }

    pub fn persist_cfg(&self) {
        let _ = save_notes_config(&self.paths.notes_cfg, &self.cfg);
    }

    pub fn sync_ui(&mut self, ui: &NotesWindow, sync_editor_fields: bool) {
        self.ensure_selection();
        sync_notes_window_ui(ui, &self.db, &self.cfg, &self.search, sync_editor_fields);
    }

    pub fn schedule_autosave(&mut self, rc: &Rc<RefCell<Self>>) {
        self.dirty = true;
        let weak_st = Rc::downgrade(rc);
        self.autosave_timer
            .start(TimerMode::SingleShot, Duration::from_millis(450), move || {
                if let Some(st) = weak_st.upgrade() {
                    let mut g = st.borrow_mut();
                    if g.dirty {
                        g.persist_db();
                    }
                }
            });
    }
}

pub fn emit_notes_cmd(action: &str, arg: &str) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", encode_ui_cmd(action, arg));
    let _ = out.flush();
}

pub fn sync_notes_window_ui(
    ui: &NotesWindow,
    db: &NotesDb,
    cfg: &NotesWidgetConfig,
    search: &str,
    sync_editor_fields: bool,
) {
    ui.set_show_folders(cfg.show_folders_pane);
    ui.set_show_list(cfg.show_list_pane);
    ui.set_show_editor(cfg.show_editor_pane);

    let folder_rows = build_folder_rows(db, cfg);
    ui.set_folders(ModelRc::from(Rc::new(VecModel::from(folder_rows))));

    let note_rows = build_note_rows(db, cfg, search);
    ui.set_notes(ModelRc::from(Rc::new(VecModel::from(note_rows))));

    if let Some(note) = db.note(cfg.active_note_id) {
        ui.set_has_active_note(true);
        ui.set_active_note_updated(SharedString::from(note.updated_at.as_str()));
        if sync_editor_fields {
            ui.set_active_note_title(SharedString::from(note.title.as_str()));
            ui.set_active_note_body(SharedString::from(note.body.as_str()));
        }
        let task_rows: Vec<SlintTaskRow> = note
            .tasks
            .iter()
            .map(|t| SlintTaskRow {
                id: t.id as i32,
                text: SharedString::from(t.text.as_str()),
                done: t.done,
            })
            .collect();
        ui.set_tasks(ModelRc::from(Rc::new(VecModel::from(task_rows))));
    } else {
        ui.set_has_active_note(false);
        ui.set_active_note_updated(SharedString::new());
        if sync_editor_fields {
            ui.set_active_note_title(SharedString::new());
            ui.set_active_note_body(SharedString::new());
        }
        ui.set_tasks(ModelRc::from(Rc::new(VecModel::<SlintTaskRow>::default())));
    }
}

pub fn build_folder_rows(db: &NotesDb, cfg: &NotesWidgetConfig) -> Vec<SlintFolderRow> {
    let mut rows = Vec::new();
    for (fid, depth, has_children) in db.visible_folder_rows() {
        let Some(f) = db.folders.iter().find(|x| x.id == fid) else {
            continue;
        };
        let count = db.notes.iter().filter(|n| n.folder_id == fid).count();
        rows.push(SlintFolderRow {
            id: fid as i32,
            name: SharedString::from(f.name.as_str()),
            depth: i32::from(depth),
            expanded: f.expanded,
            has_children: has_children || count > 0,
            active: cfg.active_folder_id == fid,
            note_count: count as i32,
            is_note_child: false,
            note_id: 0,
        });
        if f.expanded {
            for note in db.notes.iter().filter(|n| n.folder_id == fid).take(8) {
                rows.push(SlintFolderRow {
                    id: fid as i32,
                    name: SharedString::from(note.title.as_str()),
                    depth: i32::from(depth) + 1,
                    expanded: false,
                    has_children: false,
                    active: cfg.active_note_id == note.id,
                    note_count: 0,
                    is_note_child: true,
                    note_id: note.id as i32,
                });
            }
        }
    }
    rows
}

pub fn build_note_rows(
    db: &NotesDb,
    cfg: &NotesWidgetConfig,
    search: &str,
) -> Vec<SlintNoteCardRow> {
    db.filtered_notes(cfg.active_folder_id, search)
        .into_iter()
        .map(|n| {
            let (done, total) = n.task_counts();
            SlintNoteCardRow {
                id: n.id as i32,
                title: SharedString::from(n.title.as_str()),
                updated_at: SharedString::from(n.updated_at.as_str()),
                task_badge: SharedString::from(format!("{done}/{total}")),
                has_tasks: total > 0,
                pinned: n.pinned,
                active: cfg.active_note_id == n.id,
            }
        })
        .collect()
}

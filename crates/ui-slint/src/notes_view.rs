use std::cell::RefCell;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use asrbox_mod_about::AppPaths;
use asrbox_mod_widgets::{save_notes_config, save_notes_db, NotesDb, NotesWidgetConfig};
use asrbox_platform_api::{FluentMenuItem, FluentMenuSpec, WidgetLayer};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::generated::ui::{NotesWindow, SlintFolderRow, SlintNoteCardRow, SlintTaskRow};
use crate::ipc_proto::encode_ui_cmd;
use crate::vibrancy::{apply_window_opacity, set_window_layer};

pub struct LocalNotesState {
    pub paths: AppPaths,
    pub db_path: PathBuf,
    pub cfg: NotesWidgetConfig,
    pub db: NotesDb,
    pub search: String,
    pub dirty: bool,
    pub last_dark: bool,
    pub ignore_bounds_until: Instant,
    pub autosave_timer: Timer,
}

impl LocalNotesState {
    pub fn ensure_selection(&mut self) {
        if !self
            .db
            .folders
            .iter()
            .any(|f| f.id == self.cfg.active_folder_id)
        {
            self.cfg.active_folder_id = self.db.folders.first().map_or(1, |f| f.id);
        }
        let visible = self
            .db
            .filtered_notes(self.cfg.active_folder_id, &self.search);
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
        self.autosave_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(450),
            move || {
                if let Some(st) = weak_st.upgrade() {
                    let mut g = st.borrow_mut();
                    if g.dirty {
                        g.persist_db();
                    }
                }
            },
        );
    }
}

pub fn emit_notes_cmd(action: &str, arg: &str) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", encode_ui_cmd(action, arg));
    let _ = out.flush();
}

pub fn build_notes_context_menu(cfg: &NotesWidgetConfig) -> FluentMenuSpec {
    let layers = vec![
        menu_act(10, "Над обоями", cfg.layer == WidgetLayer::Wallpaper),
        menu_act(11, "Обычное окно", cfg.layer == WidgetLayer::Normal),
        menu_act(12, "Поверх всех окон", cfg.layer == WidgetLayer::Topmost),
    ];
    let opacities = vec![
        menu_act(20, "100%", cfg.opacity_percent >= 96),
        menu_act(21, "90%", (86..96).contains(&cfg.opacity_percent)),
        menu_act(22, "80%", (73..86).contains(&cfg.opacity_percent)),
        menu_act(23, "65%", (58..73).contains(&cfg.opacity_percent)),
        menu_act(24, "50%", cfg.opacity_percent < 58),
    ];
    let panes = vec![
        menu_act(30, "Папки (Ctrl+1)", cfg.show_folders_pane),
        menu_act(31, "Список заметок (Ctrl+2)", cfg.show_list_pane),
        menu_act(32, "Редактор (Ctrl+3)", cfg.show_editor_pane),
    ];
    FluentMenuSpec {
        quick_actions: Vec::new(),
        min_width: 210,
        items: vec![
            FluentMenuItem::Submenu {
                icon: Some("◫".to_owned()),
                label: "Слой отображения".to_owned(),
                enabled: true,
                items: layers,
            },
            FluentMenuItem::Submenu {
                icon: Some("◐".to_owned()),
                label: "Прозрачность".to_owned(),
                enabled: true,
                items: opacities,
            },
            FluentMenuItem::Submenu {
                icon: Some("⊞".to_owned()),
                label: "Колонки".to_owned(),
                enabled: true,
                items: panes,
            },
            FluentMenuItem::Separator,
            menu_icon_act(40, "Папка заметок", "📂"),
            menu_icon_act(41, "Настройки заметок", "⚙"),
            FluentMenuItem::Separator,
            menu_icon_act(42, "Скрыть заметки", "✕"),
        ],
    }
}

pub fn apply_notes_menu_choice(g: &mut LocalNotesState, ui: &NotesWindow, cmd: u32) {
    match cmd {
        10..=12 => {
            g.cfg.layer = match cmd {
                10 => WidgetLayer::Wallpaper,
                12 => WidgetLayer::Topmost,
                _ => WidgetLayer::Normal,
            };
            set_window_layer(ui.window(), g.cfg.layer);
            g.persist_cfg();
            emit_notes_cmd("notes_changed", "");
        }
        20..=24 => {
            let pct = [100u8, 90, 80, 65, 50][(cmd - 20) as usize];
            g.cfg.opacity_percent = pct;
            apply_window_opacity(ui.window(), pct);
            g.persist_cfg();
            emit_notes_cmd("notes_changed", "");
        }
        30..=32 => {
            if g.cfg.toggle_pane((cmd - 30) as u8) {
                g.persist_cfg();
                g.sync_ui(ui, false);
                emit_notes_cmd("notes_changed", "");
            }
        }
        40 => emit_notes_cmd("open_notes_folder", ""),
        41 => emit_notes_cmd("open_notes_settings", ""),
        42 => {
            if g.dirty {
                g.persist_db();
            }
            g.cfg.visible = false;
            g.persist_cfg();
            emit_notes_cmd("notes_closed", "");
            let _ = slint::quit_event_loop();
        }
        _ => {}
    }
}

fn menu_act(id: u32, label: &str, checked: bool) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        icon: None,
        label: label.to_owned(),
        shortcut: None,
        checked,
        enabled: true,
    }
}

fn menu_icon_act(id: u32, label: &str, icon: &str) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        icon: Some(icon.to_owned()),
        label: label.to_owned(),
        shortcut: None,
        checked: false,
        enabled: true,
    }
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

use std::cell::RefCell;
use std::io::{self, BufRead, BufReader};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use asrbox_mod_about::AppPaths;
use asrbox_mod_widgets::{
    load_notes_config, load_notes_db, save_notes_config, FolderNode, NoteDocument, TaskItem,
};
use asrbox_platform_api::load_appearance_config;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel, Weak};

use crate::generated::ui::NotesWindow;
use crate::notes_view::{build_note_rows, emit_notes_cmd, LocalNotesState};
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    apply_window_vibrancy, minimize_window, select_winit_backend, start_window_drag,
    toggle_maximize_window,
};

pub fn run_notes_window(paths: AppPaths, app_version: &str) -> Result<(), String> {
    select_winit_backend()?;
    let ui = NotesWindow::new().map_err(|e| format!("create NotesWindow: {e}"))?;
    ui.set_app_version(SharedString::from(app_version));

    let app = load_appearance_config(&paths.appearance_cfg);
    let initial_dark = apply_theme_to_window(&ui, app);

    let db_path = paths.notes_dir.join("notes_db.cfg");
    let mut cfg = load_notes_config(&paths.notes_cfg).unwrap_or_default();
    cfg.visible = true;
    cfg.ensure_at_least_one_pane();
    let _ = save_notes_config(&paths.notes_cfg, &cfg);
    let db = load_notes_db(&db_path);

    let state = Rc::new(RefCell::new(LocalNotesState {
        paths,
        db_path,
        cfg,
        db,
        search: String::new(),
        dirty: false,
        last_dark: initial_dark,
        autosave_timer: Timer::default(),
    }));
    state.borrow_mut().sync_ui(&ui, true);

    wire_notes_chrome(&ui, &state);
    wire_notes_folder_callbacks(&ui, &state);
    wire_notes_list_callbacks(&ui, &state);
    wire_notes_text_callbacks(&ui, &state);
    wire_notes_task_callbacks(&ui, &state);
    spawn_notes_stdin_listener(ui.as_weak(), &state);

    let weak_vib = ui.as_weak();
    Timer::single_shot(Duration::from_millis(24), move || {
        if let Some(win) = weak_vib.upgrade() {
            apply_window_vibrancy(win.window(), initial_dark);
        }
    });

    ui.run().map_err(|e| format!("run NotesWindow: {e}"))
}

fn wire_notes_chrome(ui: &NotesWindow, state: &Rc<RefCell<LocalNotesState>>) {
    let w = ui.as_weak();
    ui.on_drag_window(move || {
        if let Some(u) = w.upgrade() {
            start_window_drag(u.window());
        }
    });
    let w = ui.as_weak();
    ui.on_minimize_window(move || {
        if let Some(u) = w.upgrade() {
            minimize_window(u.window());
        }
    });
    let w = ui.as_weak();
    ui.on_maximize_window(move || {
        if let Some(u) = w.upgrade() {
            toggle_maximize_window(u.window());
        }
    });
    let st = Rc::clone(state);
    ui.on_close_window(move || {
        let mut g = st.borrow_mut();
        if g.dirty {
            g.persist_db();
        }
        g.cfg.visible = false;
        g.persist_cfg();
        emit_notes_cmd("notes_closed", "");
        let _ = slint::quit_event_loop();
    });
}

fn wire_notes_folder_callbacks(ui: &NotesWindow, state: &Rc<RefCell<LocalNotesState>>) {
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_toggle_pane(move |pane| {
        let mut g = st.borrow_mut();
        if g.cfg.toggle_pane(pane.clamp(0, 2) as u8) {
            g.persist_cfg();
            if let Some(u) = w.upgrade() {
                g.sync_ui(&u, false);
            }
            emit_notes_cmd("notes_changed", "");
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_select_folder(move |fid| {
        let mut g = st.borrow_mut();
        g.cfg.active_folder_id = fid.max(0) as u64;
        g.persist_cfg();
        if let Some(u) = w.upgrade() {
            g.sync_ui(&u, true);
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_toggle_folder_expand(move |fid| {
        let mut g = st.borrow_mut();
        if let Some(f) = g.db.folders.iter_mut().find(|f| f.id == fid.max(0) as u64) {
            f.expanded = !f.expanded;
            g.persist_db();
            if let Some(u) = w.upgrade() {
                g.sync_ui(&u, false);
            }
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_create_folder(move |name| {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return;
        }
        let mut g = st.borrow_mut();
        let id = g.db.alloc_id();
        g.db.folders.push(FolderNode {
            id,
            parent_id: None,
            name: trimmed.to_owned(),
            expanded: true,
        });
        g.cfg.active_folder_id = id;
        g.persist_db();
        g.persist_cfg();
        if let Some(u) = w.upgrade() {
            g.sync_ui(&u, true);
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_delete_folder(move |fid| {
        let mut g = st.borrow_mut();
        if g.db.folders.len() <= 1 {
            return;
        }
        let target = fid.max(0) as u64;
        g.db.folders.retain(|f| f.id != target);
        g.db.notes.retain(|n| n.folder_id != target);
        g.persist_db();
        if let Some(u) = w.upgrade() {
            g.sync_ui(&u, true);
        }
    });
}

fn wire_notes_list_callbacks(ui: &NotesWindow, state: &Rc<RefCell<LocalNotesState>>) {
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_select_note(move |nid| {
        let mut g = st.borrow_mut();
        if g.dirty {
            g.persist_db();
        }
        let id = nid.max(0) as u64;
        if let Some(n) = g.db.note(id) {
            g.cfg.active_folder_id = n.folder_id;
            g.cfg.active_note_id = id;
            g.persist_cfg();
            if let Some(u) = w.upgrade() {
                g.sync_ui(&u, true);
            }
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_create_note(move || {
        let mut g = st.borrow_mut();
        let id = g.db.alloc_id();
        let folder_id = g.cfg.active_folder_id;
        g.db.notes.push(NoteDocument {
            id,
            folder_id,
            title: "Новая заметка".to_owned(),
            body: String::new(),
            tasks: Vec::new(),
            updated_at: "Сегодня".to_owned(),
            pinned: false,
        });
        g.cfg.active_note_id = id;
        g.persist_db();
        g.persist_cfg();
        if let Some(u) = w.upgrade() {
            g.sync_ui(&u, true);
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_delete_note(move |nid| {
        let mut g = st.borrow_mut();
        g.db.notes.retain(|n| n.id != nid.max(0) as u64);
        g.persist_db();
        if let Some(u) = w.upgrade() {
            g.sync_ui(&u, true);
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_toggle_note_pin(move |nid| {
        let mut g = st.borrow_mut();
        if let Some(n) = g.db.note_mut(nid.max(0) as u64) {
            n.pinned = !n.pinned;
            g.persist_db();
            if let Some(u) = w.upgrade() {
                g.sync_ui(&u, false);
            }
        }
    });
}

fn wire_notes_text_callbacks(ui: &NotesWindow, state: &Rc<RefCell<LocalNotesState>>) {
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_search_changed(move |q| {
        let mut g = st.borrow_mut();
        g.search = q.to_string();
        if let Some(u) = w.upgrade() {
            g.sync_ui(&u, true);
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_title_edited(move |txt| {
        let mut g = st.borrow_mut();
        let id = g.cfg.active_note_id;
        if let Some(n) = g.db.note_mut(id) {
            n.title = txt.to_string();
            n.updated_at = "Изменено".to_owned();
            if let Some(u) = w.upgrade() {
                let rows = build_note_rows(&g.db, &g.cfg, &g.search);
                u.set_notes(ModelRc::from(Rc::new(VecModel::from(rows))));
            }
            g.schedule_autosave(&st);
        }
    });
    let st = Rc::clone(state);
    ui.on_body_edited(move |txt| {
        let mut g = st.borrow_mut();
        let id = g.cfg.active_note_id;
        if let Some(n) = g.db.note_mut(id) {
            n.body = txt.to_string();
            n.updated_at = "Изменено".to_owned();
            g.schedule_autosave(&st);
        }
    });
}

fn wire_notes_task_callbacks(ui: &NotesWindow, state: &Rc<RefCell<LocalNotesState>>) {
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_add_task(move |txt| {
        let trimmed = txt.trim();
        if trimmed.is_empty() {
            return;
        }
        let mut g = st.borrow_mut();
        let tid = g.db.alloc_id();
        let nid = g.cfg.active_note_id;
        if let Some(n) = g.db.note_mut(nid) {
            n.tasks.push(TaskItem {
                id: tid,
                text: trimmed.to_owned(),
                done: false,
            });
            g.persist_db();
            if let Some(u) = w.upgrade() {
                g.sync_ui(&u, false);
            }
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_toggle_task(move |tid| {
        let mut g = st.borrow_mut();
        let nid = g.cfg.active_note_id;
        if let Some(n) = g.db.note_mut(nid) {
            if let Some(t) = n.tasks.iter_mut().find(|t| t.id == tid.max(0) as u64) {
                t.done = !t.done;
                g.persist_db();
                if let Some(u) = w.upgrade() {
                    g.sync_ui(&u, false);
                }
            }
        }
    });
    let (w, st) = (ui.as_weak(), Rc::clone(state));
    ui.on_delete_task(move |tid| {
        let mut g = st.borrow_mut();
        let nid = g.cfg.active_note_id;
        if let Some(n) = g.db.note_mut(nid) {
            n.tasks.retain(|t| t.id != tid.max(0) as u64);
            g.persist_db();
            if let Some(u) = w.upgrade() {
                g.sync_ui(&u, false);
            }
        }
    });
}

thread_local! {
    static NOTES_STATE_HOOK: RefCell<Option<Rc<RefCell<LocalNotesState>>>> =
        const { RefCell::new(None) };
}

fn spawn_notes_stdin_listener(weak: Weak<NotesWindow>, state: &Rc<RefCell<LocalNotesState>>) {
    NOTES_STATE_HOOK.with(|slot| *slot.borrow_mut() = Some(Rc::clone(state)));
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in BufReader::new(stdin.lock()).lines().map_while(Result::ok) {
            let trimmed = line.trim().to_owned();
            if trimmed == "SHUTDOWN" {
                let _ = slint::invoke_from_event_loop(|| {
                    NOTES_STATE_HOOK.with(|slot| {
                        if let Some(rc) = slot.borrow().as_ref() {
                            let mut g = rc.borrow_mut();
                            if g.dirty {
                                g.persist_db();
                            }
                        }
                    });
                    let _ = slint::quit_event_loop();
                });
                return;
            }
            if trimmed == "REFRESH" {
                let w = weak.clone();
                let _ = slint::invoke_from_event_loop(move || refresh_notes_from_disk(&w));
            }
        }
        let _ = slint::invoke_from_event_loop(|| {
            let _ = slint::quit_event_loop();
        });
    });
}

fn refresh_notes_from_disk(weak: &Weak<NotesWindow>) {
    let Some(ui) = weak.upgrade() else {
        return;
    };
    NOTES_STATE_HOOK.with(|slot| {
        let Some(rc) = slot.borrow().as_ref().cloned() else {
            return;
        };
        let mut g = rc.borrow_mut();
        let app = load_appearance_config(&g.paths.appearance_cfg);
        let dark = apply_theme_to_window(&ui, app);
        if dark != g.last_dark {
            g.last_dark = dark;
            apply_window_vibrancy(ui.window(), dark);
        }
        if let Ok(fresh) = load_notes_config(&g.paths.notes_cfg) {
            g.cfg.show_folders_pane = fresh.show_folders_pane;
            g.cfg.show_list_pane = fresh.show_list_pane;
            g.cfg.show_editor_pane = fresh.show_editor_pane;
            g.cfg.layer = fresh.layer;
            g.cfg.opacity_percent = fresh.opacity_percent;
            g.cfg.ensure_at_least_one_pane();
        }
        g.sync_ui(&ui, false);
    });
}

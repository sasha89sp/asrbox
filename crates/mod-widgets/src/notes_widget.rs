use std::path::PathBuf;

use asrbox_platform_api::{
    FluentPalette, NotesUiEvent, PixelPoint, PixelRect, WidgetLayer, WidgetMenuVisibility,
    WidgetPlatform,
};

use crate::notes_model::{
    FolderNode, NoteDocument, NotesDb, NotesWidgetConfig, TaskItem, MAX_NOTES_HEIGHT,
    MAX_NOTES_WIDTH, MIN_NOTES_HEIGHT, MIN_NOTES_WIDTH,
};
use crate::notes_render::{
    hit_test_notes, render_notes_widget, InlineEditMode, NotesHitBox, NotesHitTarget,
    NotesRenderInput,
};
use crate::notes_storage::{load_notes_config, load_notes_db, save_notes_config, save_notes_db};

pub const NOTES_WIDGET_ID: u64 = 4;

pub struct NotesWidget {
    config_path: PathBuf,
    db_path: PathBuf,
    pub(crate) config: NotesWidgetConfig,
    pub(crate) db: NotesDb,
    window_open: bool,
    search_query: String,
    hovered: Option<NotesHitTarget>,
    pub(crate) hits: Vec<NotesHitBox>,
    list_scroll: i32,
    pub(crate) inline_mode: InlineEditMode,
    pub(crate) palette: FluentPalette,
    pub(crate) menu_vis: WidgetMenuVisibility,
}

impl NotesWidget {
    pub fn load(config_path: PathBuf, db_path: PathBuf) -> Self {
        let mut config = load_notes_config(&config_path).unwrap_or_default();
        let db = load_notes_db(&db_path);
        if !db.folders.iter().any(|f| f.id == config.active_folder_id) {
            config.active_folder_id = db.folders.first().map_or(1, |f| f.id);
        }
        if !db.notes.iter().any(|n| n.id == config.active_note_id) {
            config.active_note_id = db.notes.first().map_or(0, |n| n.id);
        }
        Self {
            config_path,
            db_path,
            config,
            db,
            window_open: false,
            search_query: String::new(),
            hovered: None,
            hits: Vec::new(),
            list_scroll: 0,
            inline_mode: InlineEditMode::None,
            palette: FluentPalette::default(),
            menu_vis: WidgetMenuVisibility::default(),
        }
    }

    pub fn set_menu_visibility(&mut self, vis: WidgetMenuVisibility) {
        self.menu_vis = vis;
    }

    pub fn set_palette(&mut self, host: &mut dyn WidgetPlatform, palette: FluentPalette) {
        self.palette = palette;
        if self.window_open {
            host.apply_notes_theme(palette);
            self.redraw(host);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    pub fn config(&self) -> &NotesWidgetConfig {
        &self.config
    }

    pub fn status_summary(&self) -> String {
        format!(
            "visible={}, {}x{}, folders={}, notes={}",
            self.config.visible,
            self.config.width,
            self.config.height,
            self.db.folders.len(),
            self.db.notes.len()
        )
    }

    pub fn sync_window(
        &mut self,
        host: &mut dyn WidgetPlatform,
        safe_mode: bool,
    ) -> Result<(), String> {
        if !safe_mode && self.config.visible {
            if !self.window_open {
                host.open_notes_window(
                    "Заметки — asrbox",
                    self.window_rect(),
                    self.config.layer,
                    self.config.opacity_percent,
                    self.palette,
                )?;
                self.window_open = true;
                self.sync_inputs_to_active_note(host);
            } else {
                host.set_notes_layer(self.config.layer);
                host.set_notes_opacity(self.config.opacity_percent);
            }
            self.redraw(host);
        } else if self.window_open {
            host.close_notes_window();
            self.window_open = false;
        }
        Ok(())
    }

    pub fn toggle_visible(&mut self, host: &mut dyn WidgetPlatform, safe_mode: bool) {
        self.config.visible = !self.config.visible;
        self.persist_cfg();
        let _ = self.sync_window(host, safe_mode);
    }

    pub fn reload_config(&mut self, host: &mut dyn WidgetPlatform, safe_mode: bool) -> String {
        match load_notes_config(&self.config_path) {
            Ok(cfg) => self.config = cfg,
            Err(e) => return format!("error: reload notes.cfg: {e}"),
        }
        let _ = self.sync_window(host, safe_mode);
        format!("reload notes.cfg: {}", self.status_summary())
    }

    pub fn handle_event(&mut self, host: &mut dyn WidgetPlatform, ev: NotesUiEvent) -> String {
        match ev {
            NotesUiEvent::PointerMove { x, y } => {
                let next = hit_test_notes(&self.hits, x, y);
                if next != self.hovered {
                    self.hovered = next;
                    self.redraw(host);
                }
            }
            NotesUiEvent::MouseDown { x, y } => return self.on_click(host, x, y),
            NotesUiEvent::MouseUp { .. } => {}
            NotesUiEvent::RightClick {
                x,
                y,
                screen_x,
                screen_y,
            } => {
                return self.on_right_click(
                    host,
                    x,
                    y,
                    PixelPoint {
                        x: screen_x,
                        y: screen_y,
                    },
                );
            }
            NotesUiEvent::Wheel { delta, .. } => {
                self.list_scroll = (self.list_scroll - i32::from(delta) / 3).max(0);
                self.redraw(host);
            }
            NotesUiEvent::Moved { x, y } => {
                if self.config.x != x || self.config.y != y {
                    self.config.x = x;
                    self.config.y = y;
                    self.persist_cfg();
                }
            }
            NotesUiEvent::Resized {
                x,
                y,
                width,
                height,
                finished,
            } => {
                self.config.x = x;
                self.config.y = y;
                self.config.width = width.clamp(MIN_NOTES_WIDTH, MAX_NOTES_WIDTH);
                self.config.height = height.clamp(MIN_NOTES_HEIGHT, MAX_NOTES_HEIGHT);
                self.redraw(host);
                if finished {
                    self.persist_cfg();
                }
            }
            NotesUiEvent::InputChanged { slot, text } => self.on_input_changed(host, slot, text),
            NotesUiEvent::InlineSubmit => self.commit_inline_edit(host),
            NotesUiEvent::InlineCancel => {
                self.inline_mode = InlineEditMode::None;
                self.redraw(host);
            }
            NotesUiEvent::TogglePane(idx) => {
                if self.config.toggle_pane(idx) {
                    self.persist_cfg();
                    self.redraw(host);
                }
            }
            NotesUiEvent::Closed => {
                self.window_open = false;
                self.config.visible = false;
                self.persist_cfg();
                return "notes closed".to_owned();
            }
        }
        String::new()
    }

    fn on_click(&mut self, host: &mut dyn WidgetPlatform, x: i32, y: i32) -> String {
        let Some(target) = hit_test_notes(&self.hits, x, y) else {
            return String::new();
        };
        match target {
            NotesHitTarget::TogglePane(idx) => {
                if self.config.toggle_pane(idx) {
                    self.persist_cfg();
                    self.redraw(host);
                }
            }
            NotesHitTarget::SettingsBtn => return "open_settings_notes".to_owned(),
            NotesHitTarget::CloseBtn => {
                self.config.visible = false;
                self.persist_cfg();
                let _ = self.sync_window(host, false);
                return "notes hidden".to_owned();
            }
            NotesHitTarget::AddFolderBtn => self.start_inline(host, InlineEditMode::NewFolder),
            NotesHitTarget::FolderChevron(fid) => {
                if let Some(f) = self.db.folders.iter_mut().find(|f| f.id == fid) {
                    f.expanded = !f.expanded;
                    self.persist_db();
                    self.redraw(host);
                }
            }
            NotesHitTarget::FolderRow(fid) => {
                self.config.active_folder_id = fid;
                self.list_scroll = 0;
                if let Some(first) = self.db.notes.iter().find(|n| n.folder_id == fid) {
                    self.config.active_note_id = first.id;
                    self.sync_inputs_to_active_note(host);
                }
                self.persist_cfg();
                self.redraw(host);
            }
            NotesHitTarget::AddNoteBtn => self.create_new_note(host, false),
            NotesHitTarget::AddTaskQuickBtn => {
                if self.db.note(self.config.active_note_id).is_none() {
                    self.create_new_note(host, true);
                } else {
                    self.start_inline(host, InlineEditMode::NewTask);
                }
            }
            NotesHitTarget::NoteCard(nid) => {
                self.config.active_note_id = nid;
                self.persist_cfg();
                self.sync_inputs_to_active_note(host);
                self.redraw(host);
            }
            NotesHitTarget::TaskCheck(tid) => {
                if let Some(note) = self.db.note_mut(self.config.active_note_id) {
                    if let Some(t) = note.tasks.iter_mut().find(|t| t.id == tid) {
                        t.done = !t.done;
                    }
                }
                self.persist_db();
                self.redraw(host);
            }
            NotesHitTarget::TaskDelete(tid) => {
                if let Some(note) = self.db.note_mut(self.config.active_note_id) {
                    note.tasks.retain(|t| t.id != tid);
                }
                self.persist_db();
                self.redraw(host);
            }
            NotesHitTarget::AddTaskBtn => self.start_inline(host, InlineEditMode::NewTask),
        }
        String::new()
    }

    fn start_inline(&mut self, host: &mut dyn WidgetPlatform, mode: InlineEditMode) {
        self.inline_mode = mode;
        host.set_notes_input_text(3, "");
        self.redraw(host);
        host.focus_notes_input(3);
    }

    fn create_new_note(&mut self, host: &mut dyn WidgetPlatform, start_task_inline: bool) {
        let now = host.local_time_now();
        let nid = self.db.alloc_id();
        self.db.notes.push(NoteDocument {
            id: nid,
            folder_id: self.config.active_folder_id,
            title: "Новая заметка".to_owned(),
            body: String::new(),
            tasks: Vec::new(),
            updated_at: format!("{:02}.{:02}.{:04}", now.day, now.month, now.year),
            pinned: false,
        });
        self.config.active_note_id = nid;
        self.persist_db();
        self.persist_cfg();
        self.sync_inputs_to_active_note(host);
        if start_task_inline {
            self.start_inline(host, InlineEditMode::NewTask);
        } else {
            self.redraw(host);
            host.focus_notes_input(1);
        }
    }

    fn on_input_changed(&mut self, host: &mut dyn WidgetPlatform, slot: u8, text: String) {
        let now = host.local_time_now();
        let stamp = format!(
            "{:02}.{:02} {:02}:{:02}",
            now.day, now.month, now.hour, now.minute
        );
        match slot {
            0 => {
                self.search_query = text;
                self.list_scroll = 0;
                self.redraw(host);
            }
            1 => {
                if let Some(n) = self.db.note_mut(self.config.active_note_id) {
                    n.title = text;
                    n.updated_at = stamp;
                }
                self.persist_db();
                self.redraw(host);
            }
            2 => {
                if let Some(n) = self.db.note_mut(self.config.active_note_id) {
                    n.body = text;
                    n.updated_at = stamp;
                }
                self.persist_db();
            }
            _ => {}
        }
    }

    fn commit_inline_edit(&mut self, host: &mut dyn WidgetPlatform) {
        let text = host.notes_input_text(3).trim().to_owned();
        let mode = self.inline_mode;
        self.inline_mode = InlineEditMode::None;
        if !text.is_empty() {
            match mode {
                InlineEditMode::NewFolder => {
                    let fid = self.db.alloc_id();
                    self.db.folders.push(FolderNode {
                        id: fid,
                        parent_id: None,
                        name: text,
                        expanded: true,
                    });
                    self.config.active_folder_id = fid;
                    self.persist_db();
                    self.persist_cfg();
                }
                InlineEditMode::RenameFolder(fid) => {
                    if let Some(f) = self.db.folders.iter_mut().find(|f| f.id == fid) {
                        f.name = text;
                    }
                    self.persist_db();
                }
                InlineEditMode::NewTask => {
                    let tid = self.db.alloc_id();
                    if let Some(n) = self.db.note_mut(self.config.active_note_id) {
                        n.tasks.push(TaskItem {
                            id: tid,
                            text,
                            done: false,
                        });
                    }
                    self.persist_db();
                }
                InlineEditMode::None => {}
            }
        }
        self.redraw(host);
    }

    pub(crate) fn sync_inputs_to_active_note(&self, host: &mut dyn WidgetPlatform) {
        host.set_notes_input_text(0, &self.search_query);
        if let Some(n) = self.db.note(self.config.active_note_id) {
            host.set_notes_input_text(1, &n.title);
            host.set_notes_input_text(2, &n.body);
        } else {
            host.set_notes_input_text(1, "");
            host.set_notes_input_text(2, "");
        }
    }

    pub(crate) fn set_layer(&mut self, host: &mut dyn WidgetPlatform, layer: WidgetLayer) {
        self.config.layer = layer;
        host.set_notes_layer(layer);
        self.persist_cfg();
    }

    pub(crate) fn redraw(&mut self, host: &mut dyn WidgetPlatform) {
        if !self.window_open {
            return;
        }
        let out = render_notes_widget(&NotesRenderInput {
            cfg: &self.config,
            db: &self.db,
            search_query: &self.search_query,
            hovered: self.hovered,
            list_scroll: self.list_scroll,
            inline_mode: self.inline_mode,
            palette: self.palette,
        });
        self.hits = out.hits;
        host.present_notes_frame(&out.frame, out.inputs);
    }

    fn window_rect(&self) -> PixelRect {
        PixelRect {
            x: self.config.x,
            y: self.config.y,
            width: self.config.width,
            height: self.config.height,
        }
    }

    pub(crate) fn persist_cfg(&self) {
        let _ = save_notes_config(&self.config_path, &self.config);
    }

    pub(crate) fn persist_db(&self) {
        let _ = save_notes_db(&self.db_path, &self.db);
    }
}

use asrbox_platform_api::{
    FluentMenuItem, FluentMenuSpec, PixelPoint, WidgetLayer, WidgetPlatform,
};

use crate::notes_model::{FolderNode, TaskItem};
use crate::notes_render::{hit_test_notes, InlineEditMode, NotesHitTarget};
use crate::notes_widget::NotesWidget;
use crate::storage::{MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT};

impl NotesWidget {
    pub(crate) fn on_right_click(
        &mut self,
        host: &mut dyn WidgetPlatform,
        x: i32,
        y: i32,
        screen: PixelPoint,
    ) -> String {
        let hit = hit_test_notes(&self.hits, x, y);
        if let Some(NotesHitTarget::FolderRow(fid)) = hit {
            let mut items = vec![
                act(1, "Переименовать папку", false),
                act(2, "Новая подпапка", false),
            ];
            if self.db.folders.len() > 1 {
                items.push(act(3, "Удалить папку", false));
            }
            let spec = FluentMenuSpec {
                items,
                min_width: 190,
                ..FluentMenuSpec::default()
            };
            if let Ok(Some(cmd)) = host.show_notes_popup_menu(screen, &spec, self.palette) {
                self.apply_folder_menu(host, fid, cmd);
            }
            return String::new();
        }
        if let Some(NotesHitTarget::NoteCard(nid)) = hit {
            let spec = FluentMenuSpec {
                items: vec![
                    act(10, "Закрепить / открепить", false),
                    act(11, "Удалить заметку", false),
                ],
                min_width: 190,
                ..FluentMenuSpec::default()
            };
            if let Ok(Some(cmd)) = host.show_notes_popup_menu(screen, &spec, self.palette) {
                self.apply_note_menu(host, nid, cmd);
            }
            return String::new();
        }
        self.show_window_context_menu(host, screen)
    }

    fn show_window_context_menu(
        &mut self,
        host: &mut dyn WidgetPlatform,
        screen: PixelPoint,
    ) -> String {
        let mut items = vec![act(20, "Настройки Заметок", false)];
        if self.menu_vis.show_layer {
            items.push(FluentMenuItem::Submenu {
                icon: None,
                label: "Слой окна".to_owned(),
                enabled: true,
                items: vec![
                    act(
                        21,
                        "На рабочем столе",
                        self.config.layer == WidgetLayer::Wallpaper,
                    ),
                    act(22, "Обычный", self.config.layer == WidgetLayer::Normal),
                    act(
                        23,
                        "Поверх всех окон",
                        self.config.layer == WidgetLayer::Topmost,
                    ),
                ],
            });
        }
        if self.menu_vis.show_opacity {
            items.push(FluentMenuItem::Submenu {
                icon: None,
                label: format!("Прозрачность ({}%)", self.config.opacity_percent),
                enabled: true,
                items: [60u8, 75, 85, 94, 100]
                    .into_iter()
                    .map(|p| {
                        act(
                            100 + u32::from(p),
                            &format!("{p}%"),
                            self.config.opacity_percent == p,
                        )
                    })
                    .collect(),
            });
        }
        if self.menu_vis.show_hide {
            items.push(FluentMenuItem::Separator);
            items.push(act(29, "Скрыть заметки", false));
        }
        let spec = FluentMenuSpec {
            items,
            min_width: 200,
            ..FluentMenuSpec::default()
        };
        if let Ok(Some(cmd)) = host.show_notes_popup_menu(screen, &spec, self.palette) {
            match cmd {
                20 => return "open_settings_notes".to_owned(),
                21 => self.set_layer(host, WidgetLayer::Wallpaper),
                22 => self.set_layer(host, WidgetLayer::Normal),
                23 => self.set_layer(host, WidgetLayer::Topmost),
                160..=200 => {
                    let pct = (cmd - 100) as u8;
                    self.config.opacity_percent =
                        pct.clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
                    host.set_notes_opacity(self.config.opacity_percent);
                    self.persist_cfg();
                }
                29 => {
                    self.config.visible = false;
                    self.persist_cfg();
                    let _ = self.sync_window(host, false);
                }
                _ => {}
            }
        }
        String::new()
    }

    fn apply_folder_menu(&mut self, host: &mut dyn WidgetPlatform, fid: u64, cmd: u32) {
        match cmd {
            1 => {
                let name = self
                    .db
                    .folders
                    .iter()
                    .find(|f| f.id == fid)
                    .map(|f| f.name.clone())
                    .unwrap_or_default();
                self.inline_mode = InlineEditMode::RenameFolder(fid);
                host.set_notes_input_text(3, &name);
                self.redraw(host);
                host.focus_notes_input(3);
            }
            2 => {
                let new_id = self.db.alloc_id();
                if let Some(p) = self.db.folders.iter_mut().find(|f| f.id == fid) {
                    p.expanded = true;
                }
                self.db.folders.push(FolderNode {
                    id: new_id,
                    parent_id: Some(fid),
                    name: "Подпапка".to_owned(),
                    expanded: true,
                });
                self.persist_db();
                self.redraw(host);
            }
            3 => {
                self.db
                    .folders
                    .retain(|f| f.id != fid && f.parent_id != Some(fid));
                if let Some(first) = self.db.folders.first() {
                    let fallback = first.id;
                    for n in &mut self.db.notes {
                        if n.folder_id == fid {
                            n.folder_id = fallback;
                        }
                    }
                    self.config.active_folder_id = fallback;
                }
                self.persist_db();
                self.persist_cfg();
                self.redraw(host);
            }
            _ => {}
        }
    }

    fn apply_note_menu(&mut self, host: &mut dyn WidgetPlatform, nid: u64, cmd: u32) {
        match cmd {
            10 => {
                if let Some(n) = self.db.note_mut(nid) {
                    n.pinned = !n.pinned;
                }
                self.persist_db();
                self.redraw(host);
            }
            11 => {
                self.db.notes.retain(|n| n.id != nid);
                if self.config.active_note_id == nid {
                    self.config.active_note_id = self.db.notes.first().map_or(0, |n| n.id);
                    self.sync_inputs_to_active_note(host);
                }
                self.persist_db();
                self.persist_cfg();
                self.redraw(host);
            }
            _ => {}
        }
    }

    pub(crate) fn commit_inline_edit(&mut self, host: &mut dyn WidgetPlatform) {
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
                InlineEditMode::EditTask(tid) => {
                    if let Some(n) = self.db.note_mut(self.config.active_note_id) {
                        if let Some(t) = n.tasks.iter_mut().find(|t| t.id == tid) {
                            t.text = text;
                        }
                    }
                    self.persist_db();
                }
                InlineEditMode::None => {}
            }
        }
        self.redraw(host);
    }
}

fn act(id: u32, label: &str, checked: bool) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        icon: None,
        label: label.to_owned(),
        shortcut: None,
        checked,
        enabled: true,
    }
}

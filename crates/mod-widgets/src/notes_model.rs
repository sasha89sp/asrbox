use asrbox_platform_api::WidgetLayer;

pub const MIN_NOTES_WIDTH: u32 = 300;
pub const MAX_NOTES_WIDTH: u32 = 1400;
pub const MIN_NOTES_HEIGHT: u32 = 320;
pub const MAX_NOTES_HEIGHT: u32 = 1000;
pub const DEFAULT_NOTES_WIDTH: u32 = 780;
pub const DEFAULT_NOTES_HEIGHT: u32 = 520;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesWidgetConfig {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub layer: WidgetLayer,
    pub opacity_percent: u8,
    pub show_folders_pane: bool,
    pub show_list_pane: bool,
    pub show_editor_pane: bool,
    pub active_folder_id: u64,
    pub active_note_id: u64,
}

impl Default for NotesWidgetConfig {
    fn default() -> Self {
        Self {
            visible: false,
            x: 160,
            y: 120,
            width: DEFAULT_NOTES_WIDTH,
            height: DEFAULT_NOTES_HEIGHT,
            layer: WidgetLayer::Normal,
            opacity_percent: 94,
            show_folders_pane: true,
            show_list_pane: true,
            show_editor_pane: true,
            active_folder_id: 1,
            active_note_id: 10,
        }
    }
}

impl NotesWidgetConfig {
    pub fn visible_pane_count(&self) -> u8 {
        u8::from(self.show_folders_pane)
            + u8::from(self.show_list_pane)
            + u8::from(self.show_editor_pane)
    }

    pub fn ensure_at_least_one_pane(&mut self) {
        if self.visible_pane_count() == 0 {
            self.show_editor_pane = true;
        }
    }

    /// Toggles pane `0` (folders), `1` (list), or `2` (editor).
    /// Returns `false` if toggling would leave 0 visible panes.
    pub fn toggle_pane(&mut self, pane: u8) -> bool {
        let cur = match pane {
            0 => self.show_folders_pane,
            1 => self.show_list_pane,
            2 => self.show_editor_pane,
            _ => return false,
        };
        if cur && self.visible_pane_count() <= 1 {
            return false;
        }
        match pane {
            0 => self.show_folders_pane = !self.show_folders_pane,
            1 => self.show_list_pane = !self.show_list_pane,
            2 => self.show_editor_pane = !self.show_editor_pane,
            _ => return false,
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub expanded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskItem {
    pub id: u64,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDocument {
    pub id: u64,
    pub folder_id: u64,
    pub title: String,
    pub body: String,
    pub tasks: Vec<TaskItem>,
    pub updated_at: String,
    pub pinned: bool,
}

impl NoteDocument {
    pub fn task_counts(&self) -> (usize, usize) {
        let done = self.tasks.iter().filter(|t| t.done).count();
        (done, self.tasks.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesDb {
    pub next_id: u64,
    pub folders: Vec<FolderNode>,
    pub notes: Vec<NoteDocument>,
}

impl Default for NotesDb {
    fn default() -> Self {
        Self::default_seeded()
    }
}

impl NotesDb {
    pub fn default_seeded() -> Self {
        Self {
            next_id: 100,
            folders: vec![
                FolderNode {
                    id: 1,
                    parent_id: None,
                    name: "Рабочие".to_owned(),
                    expanded: true,
                },
                FolderNode {
                    id: 2,
                    parent_id: None,
                    name: "Личное".to_owned(),
                    expanded: false,
                },
                FolderNode {
                    id: 3,
                    parent_id: None,
                    name: "Идеи".to_owned(),
                    expanded: false,
                },
            ],
            notes: vec![
                NoteDocument {
                    id: 10,
                    folder_id: 1,
                    title: "План разработки".to_owned(),
                    body: "Необходимо проверить настройки бэкапа с AES-256 и синхронизацию WebDAV.\n\nПодсказка: используйте Ctrl+1/2/3 для переключения колонок.".to_owned(),
                    tasks: vec![
                        TaskItem {
                            id: 21,
                            text: "Дизайн настроек ASR Glass".to_owned(),
                            done: true,
                        },
                        TaskItem {
                            id: 22,
                            text: "Виджет погоды Open-Meteo".to_owned(),
                            done: true,
                        },
                        TaskItem {
                            id: 23,
                            text: "Настроить бэкап исходников".to_owned(),
                            done: false,
                        },
                    ],
                    updated_at: "Сегодня".to_owned(),
                    pinned: true,
                },
                NoteDocument {
                    id: 11,
                    folder_id: 1,
                    title: "Быстрая заметка".to_owned(),
                    body: "Все заметки сохраняются автоматически локально в data/notes/notes_db.cfg."
                        .to_owned(),
                    tasks: Vec::new(),
                    updated_at: "Сегодня".to_owned(),
                    pinned: false,
                },
            ],
        }
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    pub fn visible_folder_rows(&self) -> Vec<(u64, u8, bool)> {
        let mut out = Vec::new();
        for f in &self.folders {
            if f.parent_id.is_none() {
                let has_children = self.folders.iter().any(|c| c.parent_id == Some(f.id));
                out.push((f.id, 0, has_children));
                if f.expanded {
                    for child in &self.folders {
                        if child.parent_id == Some(f.id) {
                            out.push((child.id, 1, false));
                        }
                    }
                }
            }
        }
        out
    }

    pub fn filtered_notes(&self, folder_id: u64, search: &str) -> Vec<&NoteDocument> {
        let q = search.trim().to_lowercase();
        let mut list: Vec<&NoteDocument> = self
            .notes
            .iter()
            .filter(|n| {
                if !q.is_empty() {
                    let in_title = n.title.to_lowercase().contains(&q);
                    let in_body = n.body.to_lowercase().contains(&q);
                    let in_tasks = n.tasks.iter().any(|t| t.text.to_lowercase().contains(&q));
                    in_title || in_body || in_tasks
                } else {
                    n.folder_id == folder_id
                }
            })
            .collect();
        list.sort_by_key(|n| (!n.pinned, std::cmp::Reverse(n.id)));
        list
    }

    pub fn note_mut(&mut self, id: u64) -> Option<&mut NoteDocument> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    pub fn note(&self, id: u64) -> Option<&NoteDocument> {
        self.notes.iter().find(|n| n.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_toggle_enforces_minimum_one_pane() {
        let mut cfg = NotesWidgetConfig::default();
        assert!(cfg.toggle_pane(0));
        assert!(cfg.toggle_pane(1));
        assert!(!cfg.toggle_pane(2));
        assert!(cfg.show_editor_pane);
        assert_eq!(cfg.visible_pane_count(), 1);
    }
}

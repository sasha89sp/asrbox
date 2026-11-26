use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use asrbox_mod_dock::{icon_request_for_item, DockItem, IconSource};
use asrbox_platform_api::IconRequest;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, SharedString};

use crate::generated::ui::SlintFolderEntryRow;
use crate::win_os::{emit_worker_log, load_shell_icon};

pub struct DockIconCache {
    base_dir: PathBuf,
    icons: HashMap<String, Image>,
    logged_failures: HashSet<String>,
}

impl DockIconCache {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            icons: HashMap::new(),
            logged_failures: HashSet::new(),
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn invalidate_item(&mut self, item: &DockItem) {
        let prefix = format!("{}|", item.id);
        self.icons.retain(|k, _| !k.starts_with(&prefix));
        self.logged_failures.retain(|k| !k.starts_with(&prefix));
    }

    pub fn get_or_load(&mut self, item: &DockItem) -> Option<Image> {
        if item.is_separator() {
            return None;
        }
        let key = cache_key(item);
        if let Some(img) = self.icons.get(&key) {
            return Some(img.clone());
        }
        let req = resolve_icon_request(icon_request_for_item(item), &self.base_dir);
        match load_shell_icon(&req, 128) {
            Ok(rgba) if rgba.width > 0 && rgba.height > 0 => {
                let img = rgba_to_image(&rgba.pixels, rgba.width, rgba.height);
                self.icons.insert(key, img.clone());
                Some(img)
            }
            Ok(_) => None,
            Err(err) => {
                if self.logged_failures.insert(key) {
                    emit_worker_log(
                        "WARN",
                        "dock",
                        format!("Иконка '{}': {err}", item.target.display()),
                    );
                }
                None
            }
        }
    }

    pub fn load_path_icon(&mut self, path: &Path) -> Option<Image> {
        let key = format!("path|{}", path.display());
        if let Some(img) = self.icons.get(&key) {
            return Some(img.clone());
        }
        let req = IconRequest::AutoShell(path.to_path_buf());
        let rgba = load_shell_icon(&req, 48).ok()?;
        if rgba.width == 0 || rgba.height == 0 {
            return None;
        }
        let img = rgba_to_image(&rgba.pixels, rgba.width, rgba.height);
        self.icons.insert(key, img.clone());
        Some(img)
    }
}

pub fn resolve_dock_target(target: &Path, base_dir: &Path) -> PathBuf {
    if target.is_absolute() || target.exists() {
        return target.to_path_buf();
    }
    let joined = base_dir.join(target);
    if joined.exists() {
        joined
    } else {
        target.to_path_buf()
    }
}

pub fn is_folder_target(path: &Path) -> bool {
    path.is_dir()
        && !path.extension().is_some_and(|e| {
            e.eq_ignore_ascii_case("exe")
                || e.eq_ignore_ascii_case("lnk")
                || e.eq_ignore_ascii_case("url")
        })
}

pub fn scan_folder_entries(dir: &Path, icons: &mut DockIconCache) -> Vec<SlintFolderEntryRow> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut raw: Vec<(bool, String, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.eq_ignore_ascii_case("desktop.ini") {
                return None;
            }
            let path = e.path();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            Some((is_dir, name, path))
        })
        .collect();
    raw.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    raw.into_iter()
        .take(28)
        .map(|(is_dir, name, path)| {
            let icon = icons.load_path_icon(&path);
            SlintFolderEntryRow {
                name: SharedString::from(name),
                path: SharedString::from(path.to_string_lossy().as_ref()),
                is_dir,
                has_icon: icon.is_some(),
                icon_img: icon.unwrap_or_default(),
            }
        })
        .collect()
}

fn rgba_to_image(pixels: &[u8], width: u32, height: u32) -> Image {
    let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(pixels, width, height);
    Image::from_rgba8(buf)
}

fn resolve_icon_request(req: IconRequest, base_dir: &Path) -> IconRequest {
    match req {
        IconRequest::AutoShell(p) => IconRequest::AutoShell(resolve_dock_target(&p, base_dir)),
        IconRequest::CustomFile(p) => IconRequest::CustomFile(resolve_dock_target(&p, base_dir)),
        IconRequest::ExeResource { path, index } => IconRequest::ExeResource {
            path: resolve_dock_target(&path, base_dir),
            index,
        },
    }
}

fn cache_key(item: &DockItem) -> String {
    let src = match &item.icon {
        IconSource::AutoShell => format!("auto:{}", item.target.display()),
        IconSource::CustomFile(p) => format!("file:{}", p.display()),
        IconSource::ExeResource { path, index } => format!("res:{}:{index}", path.display()),
    };
    format!("{}|{src}", item.id)
}

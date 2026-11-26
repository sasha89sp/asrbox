use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use asrbox_mod_dock::{
    icon_request_for_item, DockAlignment, DockConfig, DockEdge, DockItem, DockLabelMode, IconSource,
};
use asrbox_platform_api::{DockMenuVisibility, PixelRect};
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};

use crate::generated::ui::{DockWindow, SlintDockIconRow};
use crate::win_os::{idx_to_layer, layer_to_idx, load_shell_icon, resolve_exe_name};

pub struct DockIconCache {
    icons: HashMap<String, Image>,
}

impl DockIconCache {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
        }
    }

    pub fn invalidate_item(&mut self, item: &DockItem) {
        let prefix = format!("{}|", item.id);
        self.icons.retain(|k, _| !k.starts_with(&prefix));
    }

    pub fn get_or_load(&mut self, item: &DockItem, icon_size_px: u16) -> Option<Image> {
        if item.is_separator() {
            return None;
        }
        let target_px = u32::from(icon_size_px.clamp(24, 128));
        let key = cache_key(item, target_px);
        if let Some(img) = self.icons.get(&key) {
            return Some(img.clone());
        }
        let req = icon_request_for_item(item);
        let rgba = load_shell_icon(&req, target_px).ok()?;
        if rgba.width == 0 || rgba.height == 0 {
            return None;
        }
        let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            &rgba.pixels,
            rgba.width,
            rgba.height,
        );
        let img = Image::from_rgba8(buf);
        self.icons.insert(key, img.clone());
        Some(img)
    }
}

fn cache_key(item: &DockItem, px: u32) -> String {
    let src = match &item.icon {
        IconSource::AutoShell => format!("auto:{}", item.target.display()),
        IconSource::CustomFile(p) => format!("file:{}", p.display()),
        IconSource::ExeResource { path, index } => format!("res:{}:{index}", path.display()),
    };
    format!("{}|{px}|{src}", item.id)
}

fn compute_dock_size(cfg: &DockConfig) -> (u32, u32) {
    let icon = u32::from(cfg.icon_size_px.clamp(24, 128));
    let pad = u32::from(cfg.padding_px.clamp(4, 48));
    let gap = u32::from(cfg.item_spacing_px.min(48));
    let lm = cfg.label_mode.to_index();
    let tile_w = if lm == 2 { icon + 96 } else { icon + 12 };
    let tile_h = if lm == 1 { icon + 24 } else { icon + 12 };
    let horiz = cfg.edge.is_horizontal();
    let mut main_len = pad * 2;
    let mut count = 0u32;
    if !cfg.locked {
        main_len += 16;
        count += 1;
    }
    for it in &cfg.items {
        main_len += if it.is_separator() {
            10
        } else if horiz {
            tile_w
        } else {
            tile_h
        };
        count += 1;
    }
    if cfg.show_settings_button {
        main_len += icon + 12;
        count += 1;
    }
    if count > 1 {
        main_len += gap * (count - 1);
    }
    let cross_len = (if horiz { tile_h } else { tile_w }) + pad * 2;
    if horiz {
        (main_len.max(88), cross_len.max(56))
    } else {
        (cross_len.max(56), main_len.max(88))
    }
}

pub fn compute_dock_rect(cfg: &DockConfig, work_area: PixelRect) -> (i32, i32, u32, u32) {
    let (w, h) = compute_dock_size(cfg);
    // Preserve user-dragged free_pos WITHOUT snapping back to screen edges
    if let Some((fx, fy)) = cfg.free_pos {
        let min_x = work_area.x - (w as i32 / 2);
        let max_x = work_area.x + work_area.width as i32 - 32;
        let min_y = work_area.y;
        let max_y = work_area.y + work_area.height as i32 - 32;
        return (fx.clamp(min_x, max_x), fy.clamp(min_y, max_y), w, h);
    }
    let margin = i32::from(cfg.edge_margin_px);
    let offset = match cfg.alignment {
        DockAlignment::Center => 0,
        DockAlignment::Offset(o) => o,
    };
    let (x, y) = match cfg.edge {
        DockEdge::Bottom => (
            work_area.x + (work_area.width as i32 - w as i32) / 2 + offset,
            work_area.y + work_area.height as i32 - h as i32 - margin,
        ),
        DockEdge::Top => (
            work_area.x + (work_area.width as i32 - w as i32) / 2 + offset,
            work_area.y + margin,
        ),
        DockEdge::Left => (
            work_area.x + margin,
            work_area.y + (work_area.height as i32 - h as i32) / 2 + offset,
        ),
        DockEdge::Right => (
            work_area.x + work_area.width as i32 - w as i32 - margin,
            work_area.y + (work_area.height as i32 - h as i32) / 2 + offset,
        ),
    };
    (x, y, w, h)
}

pub fn sync_dock_window(
    ui: &DockWindow,
    cfg: &DockConfig,
    vis: DockMenuVisibility,
    running_exes: &HashSet<String>,
    icons: &mut DockIconCache,
    can_remove: bool,
) {
    ui.set_dock_name(SharedString::from(cfg.name.as_str()));
    ui.set_is_horizontal(cfg.edge.is_horizontal());
    ui.set_icon_size(f32::from(cfg.icon_size_px.clamp(24, 128)));
    ui.set_item_spacing(f32::from(cfg.item_spacing_px.min(48)));
    ui.set_dock_padding(f32::from(cfg.padding_px.clamp(4, 48)));
    ui.set_corner_radius(f32::from(cfg.corner_radius_px.clamp(4, 32)));
    ui.set_opacity_pct(i32::from(cfg.opacity_percent));
    ui.set_edge_index(match cfg.edge {
        DockEdge::Left => 0,
        DockEdge::Right => 1,
        DockEdge::Top => 2,
        DockEdge::Bottom => 3,
    });
    ui.set_layer_index(layer_to_idx(cfg.layer));
    ui.set_label_mode(i32::from(cfg.label_mode.to_index()));
    ui.set_locked(cfg.locked);
    ui.set_lock_items(cfg.lock_items);
    ui.set_hover_zoom(cfg.hover_zoom.enabled);
    ui.set_show_running_dot(cfg.show_running_indicator);
    ui.set_focus_running(cfg.focus_running_window);
    ui.set_launch_anim(cfg.launch_animation_enabled);
    ui.set_show_settings_button(cfg.show_settings_button);
    ui.set_can_remove_dock(can_remove);
    ui.set_vis_item_actions(vis.show_item_actions);
    ui.set_vis_add_separator(vis.show_add_separator);
    ui.set_vis_edge_layer(vis.show_edge_and_layer);
    ui.set_vis_appearance(vis.show_appearance);
    ui.set_vis_behavior(vis.show_behavior);
    ui.set_vis_lock(vis.show_lock_controls);
    ui.set_vis_dock_mgmt(vis.show_dock_management);
    let rows = build_dock_icon_rows(cfg, running_exes, icons);
    ui.set_items(ModelRc::from(Rc::new(VecModel::from(rows))));
}

fn build_dock_icon_rows(
    cfg: &DockConfig,
    running_exes: &HashSet<String>,
    icons: &mut DockIconCache,
) -> Vec<SlintDockIconRow> {
    cfg.items
        .iter()
        .map(|it| {
            let img = icons.get_or_load(it, cfg.icon_size_px);
            let is_running = !it.is_separator()
                && cfg.show_running_indicator
                && resolve_exe_name(&it.target).is_some_and(|exe| running_exes.contains(&exe));
            SlintDockIconRow {
                id: it.id as i32,
                label: SharedString::from(it.label.as_str()),
                is_separator: it.is_separator(),
                is_folder: !it.is_separator() && it.target.is_dir(),
                is_running,
                has_icon: img.is_some(),
                icon_img: img.unwrap_or_default(),
            }
        })
        .collect()
}

pub fn apply_dock_mutation(
    cfg: &mut DockConfig,
    icons: &mut DockIconCache,
    action: &str,
    arg: i32,
) -> bool {
    if let Some(res) = apply_dock_item_mutation(cfg, icons, action, arg as u64) {
        return res;
    }
    match action {
        "set_edge" => {
            cfg.edge = match arg {
                0 => DockEdge::Left,
                1 => DockEdge::Right,
                2 => DockEdge::Top,
                _ => DockEdge::Bottom,
            };
            cfg.free_pos = None;
            true
        }
        "set_layer" => {
            cfg.layer = idx_to_layer(arg);
            true
        }
        "icon_size" => {
            cfg.icon_size_px = arg.clamp(24, 128) as u16;
            true
        }
        "cycle_labels" => {
            cfg.label_mode = DockLabelMode::from_index((cfg.label_mode.to_index() + 1) % 4);
            true
        }
        "toggle_hover_zoom" => {
            cfg.hover_zoom.enabled = !cfg.hover_zoom.enabled;
            true
        }
        "toggle_running_dot" => {
            cfg.show_running_indicator = !cfg.show_running_indicator;
            true
        }
        "toggle_lock" => {
            cfg.locked = !cfg.locked;
            true
        }
        "toggle_lock_items" => {
            cfg.lock_items = !cfg.lock_items;
            true
        }
        _ => false,
    }
}

fn apply_dock_item_mutation(
    cfg: &mut DockConfig,
    icons: &mut DockIconCache,
    action: &str,
    id: u64,
) -> Option<bool> {
    Some(match action {
        "reset_icon" => {
            if let Some(it) = cfg.items.iter_mut().find(|i| i.id == id) {
                it.icon = IconSource::AutoShell;
                icons.invalidate_item(it);
                true
            } else {
                false
            }
        }
        "move_prev" if !cfg.lock_items => {
            if let Some(pos) = cfg.items.iter().position(|i| i.id == id) {
                if pos > 0 {
                    cfg.items.swap(pos, pos - 1);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
        "move_next" if !cfg.lock_items => {
            if let Some(pos) = cfg.items.iter().position(|i| i.id == id) {
                if pos + 1 < cfg.items.len() {
                    cfg.items.swap(pos, pos + 1);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
        "remove_item" if !cfg.lock_items => {
            let prev = cfg.items.len();
            cfg.items.retain(|i| i.id != id);
            cfg.items.len() != prev
        }
        "add_separator" if !cfg.lock_items => {
            let nid = cfg.next_item_id();
            cfg.items.push(DockItem::separator(nid));
            true
        }
        _ => return None,
    })
}

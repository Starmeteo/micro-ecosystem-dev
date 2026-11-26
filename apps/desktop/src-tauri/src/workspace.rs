use crate::host::{HostError, HostResult};
use crate::window_error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

const LAYOUT_LIMIT: usize = 64 * 1024;
const DEFAULT_PANEL_WIDTH: f64 = 384.0;
const DEFAULT_PANEL_HEIGHT: f64 = 216.0;
const DEFAULT_PANEL_MARGIN: f64 = 28.0;

pub struct WorkspaceStore {
    directory: PathBuf,
    operation: Mutex<()>,
}

impl WorkspaceStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory, operation: Mutex::new(()) }
    }

    pub fn load(&self) -> HostResult<Option<Value>> {
        let _operation = self.lock()?;
        self.load_locked()
    }

    fn load_locked(&self) -> HostResult<Option<Value>> {
        let file = match File::open(self.directory.join("layout.json")) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(layout_error("workspace_layout_read_failed", error)),
        };
        let mut bytes = Vec::new();
        file.take((LAYOUT_LIMIT + 1) as u64).read_to_end(&mut bytes)
            .map_err(|error| layout_error("workspace_layout_read_failed", error))?;
        if bytes.len() > LAYOUT_LIMIT {
            return Err(HostError::new("workspace_layout_read_failed", "已保存的组件布局文件太大"));
        }
        let layout: Value = serde_json::from_slice(&bytes)
            .map_err(|error| layout_error("workspace_layout_read_failed", error))?;
        if !layout.is_object() {
            return Err(HostError::new("workspace_layout_read_failed", "已保存的组件布局格式无效"));
        }
        Ok(Some(layout))
    }

    pub fn save(&self, layout: &Value) -> HostResult<()> {
        let _operation = self.lock()?;
        self.save_locked(layout)
    }

    pub fn save_panel_layout(&self, mut layout: Value) -> HostResult<Value> {
        let _operation = self.lock()?;
        let values = layout.as_object_mut().ok_or_else(invalid_layout)?;
        if let Some(record) = self.load_locked()? {
            for key in ["idle", "panelWorldId"] {
                if let Some(value) = record.get(key) { values.insert(key.into(), value.clone()); }
                else if key == "panelWorldId" { values.remove(key); }
            }
            // A drag can finish after settings were saved. Preserve those bindings while
            // accepting the drag's current geometry and per-card ordering.
            if let (Some(saved), Some(incoming)) = (record.get("items").and_then(Value::as_array), values.get_mut("items").and_then(Value::as_array_mut)) {
                for item in incoming.iter_mut().filter_map(Value::as_object_mut) {
                    let Some(stored) = saved.iter().find(|stored| stored.get("id").is_some_and(|id| item.get("id") == Some(id))) else { continue; };
                    if let Some(world) = stored.get("worldId") { item.insert("worldId".into(), world.clone()); }
                    else { item.remove("worldId"); }
                }
            }
        }
        self.save_locked(&layout)?;
        Ok(layout)
    }

    pub fn save_idle(&self, idle: Value, fallback: Value, panel_world_id: Option<String>, bindings: Option<&HashMap<String, Option<String>>>) -> HostResult<Value> {
        if !idle.is_object() {
            return Err(HostError::new("invalid_workspace_layout", "待机设置必须是 JSON 对象"));
        }
        let _operation = self.lock()?;
        let mut record = self.load_locked()?.unwrap_or(fallback);
        let values = record.as_object_mut().ok_or_else(invalid_layout)?;
        values.insert("idle".into(), idle);
        if let Some(id) = panel_world_id {
            if id.is_empty() { values.remove("panelWorldId"); }
            else { values.insert("panelWorldId".into(), Value::String(id)); }
        }
        if let (Some(bindings), Some(items)) = (bindings, values.get_mut("items").and_then(Value::as_array_mut)) {
            for item in items.iter_mut().filter_map(Value::as_object_mut) {
                if item.get("kind").and_then(Value::as_str) != Some("ecology") { continue; }
                if let Some(world) = item.get("id").and_then(Value::as_str).and_then(|id| bindings.get(id)) {
                    item.insert("worldId".into(), world.as_ref().map_or(Value::Null, |id| Value::String(id.clone())));
                }
            }
        }
        self.save_locked(&record)?;
        Ok(record)
    }

    fn save_locked(&self, layout: &Value) -> HostResult<()> {
        if !layout.is_object() {
            return Err(invalid_layout());
        }
        let bytes = serde_json::to_vec(layout)
            .map_err(|error| layout_error("invalid_workspace_layout", error))?;
        if bytes.len() > LAYOUT_LIMIT {
            return Err(HostError::new("invalid_workspace_layout", "组件布局数据太大"));
        }

        fs::create_dir_all(&self.directory)
            .map_err(|error| layout_error("workspace_layout_save_failed", error))?;
        let candidate = self.directory.join("layout.json.pending");
        let result = (|| -> io::Result<()> {
            let mut file = File::create(&candidate)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&candidate, self.directory.join("layout.json"))
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&candidate);
            return Err(layout_error("workspace_layout_save_failed", error));
        }
        Ok(())
    }

    fn lock(&self) -> HostResult<MutexGuard<'_, ()>> {
        self.operation.lock().map_err(|_| HostError::new("workspace_layout_unavailable", "组件布局暂不可用"))
    }
}

fn invalid_layout() -> HostError {
    HostError::new("invalid_workspace_layout", "组件布局必须是 JSON 对象")
}

fn layout_error(code: &'static str, error: impl std::fmt::Display) -> HostError {
    let message = match code {
        "workspace_layout_read_failed" => "读取组件布局失败",
        "workspace_layout_save_failed" => "保存组件布局失败",
        _ => "组件布局数据无效",
    };
    HostError::new(code, format!("{message}。{error}"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl ScreenRect {
    fn work_area(monitor: &Monitor) -> Self {
        let area = monitor.work_area();
        Self { x: area.position.x, y: area.position.y, width: area.size.width, height: area.size.height }
    }

    fn fit(&self, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> HostResult<PhysicalPosition<i32>> {
        if size.width > self.width || size.height > self.height {
            return Err(HostError::new("panel_bounds_outside_work_area", "组件面板超出了屏幕可用区域"));
        }
        let right = i64::from(self.x) + i64::from(self.width - size.width);
        let bottom = i64::from(self.y) + i64::from(self.height - size.height);
        Ok(PhysicalPosition::new(
            i64::from(position.x).clamp(i64::from(self.x), right).min(i64::from(i32::MAX)) as i32,
            i64::from(position.y).clamp(i64::from(self.y), bottom).min(i64::from(i32::MAX)) as i32,
        ))
    }

    fn bottom_right(&self, size: PhysicalSize<u32>, margin: u32) -> PhysicalPosition<i32> {
        let horizontal = self.width.saturating_sub(size.width).saturating_sub(margin);
        let vertical = self.height.saturating_sub(size.height).saturating_sub(margin);
        PhysicalPosition::new(
            (i64::from(self.x) + i64::from(horizontal)).min(i64::from(i32::MAX)) as i32,
            (i64::from(self.y) + i64::from(vertical)).min(i64::from(i32::MAX)) as i32,
        )
    }

    fn relative_position(&self, size: PhysicalSize<u32>, horizontal: f64, vertical: f64) -> PhysicalPosition<i32> {
        let horizontal = (f64::from(self.width.saturating_sub(size.width)) * horizontal).round() as i64;
        let vertical = (f64::from(self.height.saturating_sub(size.height)) * vertical).round() as i64;
        PhysicalPosition::new(
            (i64::from(self.x) + horizontal).min(i64::from(i32::MAX)) as i32,
            (i64::from(self.y) + vertical).min(i64::from(i32::MAX)) as i32,
        )
    }

    fn exposes_window_header(&self, window: ScreenRect, scale_factor: f64) -> bool {
        let accessible_width = ((48.0 * scale_factor).round() as u32).min(window.width);
        let header_height = ((36.0 * scale_factor).round() as u32).min(window.height);
        let visible_height = ((10.0 * scale_factor).round() as u32).min(header_height);
        let left = i64::from(window.x).max(i64::from(self.x));
        let top = i64::from(window.y).max(i64::from(self.y));
        let right = (i64::from(window.x) + i64::from(window.width)).min(i64::from(self.x) + i64::from(self.width));
        let bottom = (i64::from(window.y) + i64::from(header_height)).min(i64::from(self.y) + i64::from(self.height));
        right - left >= i64::from(accessible_width) && bottom - top >= i64::from(visible_height)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelBounds {
    #[serde(flatten)]
    rect: ScreenRect,
    scale_factor: f64,
    work_area: ScreenRect,
    monitor_name: Option<String>,
    monitor_x: i32,
    monitor_y: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PanelBoundsRequest {
    x: Option<f64>,
    y: Option<f64>,
    width: f64,
    height: f64,
    placement: Option<PanelPlacement>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PanelPlacement {
    monitor_name: Option<String>,
    monitor_x: i32,
    monitor_y: i32,
    horizontal: f64,
    vertical: f64,
}

impl PanelBoundsRequest {
    fn requested_position(&self) -> HostResult<Option<PhysicalPosition<i32>>> {
        match (self.x, self.y) {
            (None, None) => Ok(None),
            (Some(x), Some(y)) if [x, y].into_iter().all(|value| {
                (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&value)
            }) => Ok(Some(PhysicalPosition::new(x.round() as i32, y.round() as i32))),
            _ => Err(HostError::new("invalid_panel_bounds", "组件面板的屏幕坐标需要成对填写")),
        }
    }
}

impl PanelPlacement {
    fn position(&self, work_area: ScreenRect, size: PhysicalSize<u32>) -> HostResult<PhysicalPosition<i32>> {
        if ![self.horizontal, self.vertical].into_iter().all(|value| (0.0..=1.0).contains(&value)) {
            return Err(HostError::new("invalid_panel_bounds", "组件面板的相对位置必须在 0 到 1 之间"));
        }
        Ok(work_area.relative_position(size, self.horizontal, self.vertical))
    }
}

fn panel_window(app: &AppHandle) -> HostResult<WebviewWindow> {
    app.get_webview_window("panel")
        .ok_or_else(|| HostError::new("view_unavailable", "当前找不到组件面板窗口"))
}

fn monitor_for_panel(window: &WebviewWindow, requested: Option<PhysicalPosition<i32>>) -> HostResult<Monitor> {
    if let Some(point) = requested {
        if let Some(monitor) = window.monitor_from_point(f64::from(point.x), f64::from(point.y)).map_err(window_error)? {
            return Ok(monitor);
        }
    }
    if let Some(monitor) = window.current_monitor().map_err(window_error)? {
        return Ok(monitor);
    }
    window.primary_monitor().map_err(window_error)?
        .ok_or_else(|| HostError::new("work_area_unavailable", "当前找不到可放置组件面板的屏幕"))
}

fn monitor_for_saved_placement(window: &WebviewWindow, placement: &PanelPlacement) -> HostResult<Monitor> {
    let monitors = window.available_monitors().map_err(window_error)?;
    let origin = PhysicalPosition::new(placement.monitor_x, placement.monitor_y);
    let name = placement.monitor_name.as_ref();
    if let Some(monitor) = monitors.iter().find(|monitor| monitor.name() == name && monitor.position() == &origin) {
        return Ok(monitor.clone());
    }
    if let Some(name) = name {
        let mut named = monitors.iter().filter(|monitor| monitor.name() == Some(name));
        if let Some(monitor) = named.next().filter(|_| named.next().is_none()) {
            return Ok(monitor.clone());
        }
    }
    window.primary_monitor().map_err(window_error)?
        .or_else(|| monitors.into_iter().next())
        .ok_or_else(|| HostError::new("work_area_unavailable", "当前找不到可放置组件面板的屏幕"))
}

pub fn panel_bounds(app: &AppHandle) -> HostResult<PanelBounds> {
    let window = panel_window(app)?;
    let position = window.inner_position().map_err(window_error)?;
    let size = window.inner_size().map_err(window_error)?;
    let monitor = monitor_for_panel(&window, None)?;
    Ok(PanelBounds {
        rect: ScreenRect { x: position.x, y: position.y, width: size.width, height: size.height },
        scale_factor: window.scale_factor().map_err(window_error)?,
        work_area: ScreenRect::work_area(&monitor),
        monitor_name: monitor.name().cloned(),
        monitor_x: monitor.position().x,
        monitor_y: monitor.position().y,
    })
}

fn physical_size(width: f64, height: f64, scale_factor: f64) -> HostResult<PhysicalSize<u32>> {
    if [width, height].into_iter().any(|value| !(1.0..=4096.0).contains(&value)) || !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(HostError::new("invalid_panel_bounds", "组件面板尺寸必须是 1 到 4096 逻辑像素"));
    }
    Ok(PhysicalSize::new((width * scale_factor).round().max(1.0) as u32, (height * scale_factor).round().max(1.0) as u32))
}

fn size_in_work_area(size: PhysicalSize<u32>, work_area: ScreenRect) -> HostResult<PhysicalSize<u32>> {
    if work_area.width == 0 || work_area.height == 0 {
        return Err(HostError::new("work_area_unavailable", "当前屏幕没有可用的组件摆放区域"));
    }
    if size.width <= work_area.width && size.height <= work_area.height {
        return Ok(size);
    }
    let ratio = (f64::from(work_area.width) / f64::from(size.width))
        .min(f64::from(work_area.height) / f64::from(size.height));
    Ok(PhysicalSize::new(
        ((f64::from(size.width) * ratio).floor() as u32).clamp(1, work_area.width),
        ((f64::from(size.height) * ratio).floor() as u32).clamp(1, work_area.height),
    ))
}

fn apply_bounds(
    window: &WebviewWindow,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
) -> HostResult<()> {
    let current_position = window.inner_position().map_err(window_error)?;
    let current_size = window.inner_size().map_err(window_error)?;
    let old_outer = if position != current_position {
        let outer = window.outer_position().map_err(window_error)?;
        let outer_target = PhysicalPosition::new(
            position.x.saturating_add(outer.x.saturating_sub(current_position.x)),
            position.y.saturating_add(outer.y.saturating_sub(current_position.y)),
        );
        window.set_position(outer_target).map_err(window_error)?;
        Some(outer)
    } else {
        None
    };
    if size != current_size {
        if let Err(error) = window.set_size(size) {
            if let Some(outer) = old_outer {
                let _ = window.set_position(outer);
            }
            return Err(window_error(error));
        }
    }
    Ok(())
}

pub fn reveal_lost_window(window: &WebviewWindow) -> HostResult<()> {
    let position = window.inner_position().map_err(window_error)?;
    let size = window.inner_size().map_err(window_error)?;
    let scale = window.scale_factor().map_err(window_error)?;
    let rect = ScreenRect { x: position.x, y: position.y, width: size.width, height: size.height };
    let monitors = window.available_monitors().map_err(window_error)?;
    if monitors.iter().any(|monitor| ScreenRect::work_area(monitor).exposes_window_header(rect, scale)) {
        return Ok(());
    }
    let monitor = window.primary_monitor().map_err(window_error)?
        .or_else(|| monitors.into_iter().next())
        .ok_or_else(|| HostError::new("work_area_unavailable", "当前找不到可放置窗口的屏幕"))?;
    let area = ScreenRect::work_area(&monitor);
    let ratio = monitor.scale_factor() / scale;
    let desired = PhysicalSize::new(
        (f64::from(size.width) * ratio).round().max(1.0) as u32,
        (f64::from(size.height) * ratio).round().max(1.0) as u32,
    );
    let fitted = size_in_work_area(desired, area)?;
    let position = area.relative_position(fitted, 0.5, 0.5);
    apply_bounds(window, position, fitted)
}

pub fn set_panel_bounds(app: &AppHandle, bounds: PanelBoundsRequest) -> HostResult<PanelBounds> {
    let window = panel_window(app)?;
    let requested = bounds.requested_position()?;
    let monitor = match &bounds.placement {
        Some(placement) if requested.is_none() => monitor_for_saved_placement(&window, placement)?,
        Some(_) => return Err(HostError::new("invalid_panel_bounds", "相对恢复位置和桌面坐标不能同时指定")),
        None => monitor_for_panel(&window, requested)?,
    };
    let work_area = ScreenRect::work_area(&monitor);
    let size = size_in_work_area(physical_size(bounds.width, bounds.height, monitor.scale_factor())?, work_area)?;
    let position = if let Some(placement) = &bounds.placement {
        placement.position(work_area, size)?
    } else {
        requested.map_or_else(|| window.inner_position().map_err(window_error), Ok)?
    };
    let position = work_area.fit(position, size)?;
    apply_bounds(&window, position, size)?;
    panel_bounds(app)
}

pub fn anchor_default_panel(app: &AppHandle) -> HostResult<()> {
    let window = panel_window(app)?;
    let monitor = window.primary_monitor().map_err(window_error)?
        .ok_or_else(|| HostError::new("work_area_unavailable", "当前找不到主屏幕"))?;
    let scale = monitor.scale_factor();
    let work_area = ScreenRect::work_area(&monitor);
    let size = size_in_work_area(physical_size(DEFAULT_PANEL_WIDTH, DEFAULT_PANEL_HEIGHT, scale)?, work_area)?;
    let position = work_area.bottom_right(size, (DEFAULT_PANEL_MARGIN * scale).round() as u32);
    apply_bounds(&window, work_area.fit(position, size)?, size)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelPointer {
    x: i32,
    y: i32,
    primary_down: bool,
}

#[cfg(windows)]
pub fn panel_pointer() -> HostResult<PanelPointer> {
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[link(name = "User32")]
    extern "system" {
        fn GetCursorPos(point: *mut Point) -> i32;
        fn GetAsyncKeyState(virtual_key: i32) -> i16;
    }

    const VK_LBUTTON: i32 = 0x01;
    let mut point = Point { x: 0, y: 0 };
    // SAFETY: Point matches Win32 POINT and is writable for this call.
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return Err(HostError::new("panel_pointer_unavailable", io::Error::last_os_error().to_string()));
    }
    // SAFETY: VK_LBUTTON is a valid virtual key. The sign bit is the current down state.
    let primary_down = unsafe { GetAsyncKeyState(VK_LBUTTON) } < 0;
    Ok(PanelPointer { x: point.x, y: point.y, primary_down })
}

#[cfg(not(windows))]
pub fn panel_pointer() -> HostResult<PanelPointer> {
    Err(HostError::new("panel_pointer_unavailable", "当前平台无法获取桌面指针"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn panel_bounds_use_desktop_edges_and_preserve_dimensions() {
        let work_area = ScreenRect { x: -1600, y: 0, width: 1600, height: 850 };
        let size = physical_size(384.0, 216.0, 1.25).unwrap();
        assert_eq!(size, PhysicalSize::new(480, 270));
        let bottom_right = work_area.bottom_right(size, 35);
        assert_eq!(bottom_right, PhysicalPosition::new(-515, 545));
        let clamped = work_area.fit(PhysicalPosition::new(-200, -50), size).unwrap();
        assert_eq!(clamped, PhysicalPosition::new(-480, 0));
        assert!(work_area.fit(clamped, PhysicalSize::new(1601, 100)).is_err());
    }

    #[test]
    fn relative_panel_placement_reuses_remaining_travel_at_the_new_scale() {
        let saved: PanelPlacement = serde_json::from_value(json!({
            "monitorName":"Display 2", "monitorX":-1600, "monitorY":-200,
            "horizontal":0.25, "vertical":0.75,
        })).unwrap();
        let old_area = ScreenRect { x: -1600, y: -200, width: 1600, height: 850 };
        let old_size = size_in_work_area(physical_size(384.0, 216.0, 1.25).unwrap(), old_area).unwrap();
        assert_eq!(saved.position(old_area, old_size).unwrap(), PhysicalPosition::new(-1320, 235));
        let fallback_area = ScreenRect { x: 40, y: 0, width: 1400, height: 900 };
        let fallback_size = size_in_work_area(physical_size(384.0, 216.0, 1.5).unwrap(), fallback_area).unwrap();
        assert_eq!(fallback_size, PhysicalSize::new(576, 324));
        assert_eq!(saved.position(fallback_area, fallback_size).unwrap(), PhysicalPosition::new(246, 432));
        let invalid = PanelPlacement { horizontal: f64::NAN, ..saved };
        assert_eq!(invalid.position(fallback_area, fallback_size).unwrap_err().code, "invalid_panel_bounds");
    }

    #[test]
    fn an_oversized_component_group_is_scaled_as_one_unit_on_small_fallback_screen() {
        let area = ScreenRect { x: -600, y: -100, width: 600, height: 400 };
        let native_size = physical_size(1200.0, 650.0, 1.5).unwrap();
        let size = size_in_work_area(native_size, area).unwrap();
        assert_eq!(size, PhysicalSize::new(600, 325));
        let position = area.relative_position(size, 0.7, 1.0);
        assert_eq!(position, PhysicalPosition::new(-600, -25));
        assert_eq!(area.fit(position, size).unwrap(), position);
    }

    #[test]
    fn visible_window_edges_are_preserved_but_a_lost_header_is_detected() {
        let primary = ScreenRect { x: 0, y: 40, width: 1600, height: 850 };
        let secondary = ScreenRect { x: -1100, y: -50, width: 1100, height: 700 };
        let partial = ScreenRect { x: -620, y: -40, width: 800, height: 570 };
        let left = ScreenRect { x: -1000, y: 100, ..partial };
        let edge = ScreenRect { x: 1520, y: 20, ..partial };
        assert!(!primary.exposes_window_header(partial, 1.25));
        assert!(secondary.exposes_window_header(partial, 1.25));
        assert!(!primary.exposes_window_header(left, 1.25));
        assert!(secondary.exposes_window_header(left, 1.25));
        assert!(primary.exposes_window_header(edge, 1.25));
    }

    #[test]
    fn workspace_layout_loads_and_replaces_only_its_own_file() {
        let root = std::env::temp_dir().join(format!("micro-ecosystem-workspace-{}", std::process::id()));
        let store = WorkspaceStore::new(root.clone());
        let old = json!({"version":1,"components":[]});
        let new = json!({"version":1,"components":[{"id":"ecosystem","x":-20}]});
        assert_eq!(store.load().unwrap(), None);
        store.save(&old).unwrap();
        store.save(&new).unwrap();
        assert_eq!(WorkspaceStore::new(root.clone()).load().unwrap(), Some(new.clone()));
        assert_eq!(store.save(&json!(["malformed"])).unwrap_err().code, "invalid_workspace_layout");
        assert_eq!(store.load().unwrap(), Some(new));
        assert!(!root.join("layout.json.pending").exists());
        fs::remove_file(root.join("layout.json")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn idle_settings_and_panel_layout_preserve_each_others_fields() {
        let root = std::env::temp_dir().join(format!("micro-ecosystem-workspace-idle-{}", std::process::id()));
        let store = WorkspaceStore::new(root.clone());
        let initial = json!({"version":2,"mode":"combined","items":[{"kind":"ecology"}],"idle":{"mode":"clock"}});
        let idle = json!({"mode":"text","text":"休息"});
        let settings_saved = store.save_idle(idle.clone(), initial.clone(), Some("pond-example".into()), None).unwrap();
        assert_eq!(settings_saved["items"], initial["items"]);
        let panel_saved = store.save_panel_layout(json!({
            "version":2,"mode":"free","items":[{"kind":"clock","x":210}],"idle":{"mode":"clock"}
        })).unwrap();
        assert_eq!(panel_saved["idle"], idle);
        assert_eq!(panel_saved["panelWorldId"], "pond-example");
        let latest = store.save_idle(json!({"mode":"off"}), initial, None, None).unwrap();
        assert_eq!(latest["panelWorldId"], "pond-example");
        assert_eq!(latest["mode"], "free");
        assert_eq!(latest["items"], panel_saved["items"]);
        assert_eq!(latest["idle"]["mode"], "off");
        fs::remove_file(root.join("layout.json")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn card_binding_survives_a_stale_geometry_save_without_reviving_removed_cards() {
        let root = std::env::temp_dir().join(format!("micro-ecosystem-workspace-bindings-{}", std::process::id()));
        let store = WorkspaceStore::new(root.clone());
        let initial = json!({"version":2,"items":[{"id":"main","kind":"ecology"},{"id":"extra","kind":"ecology"}]});
        store.save(&initial).unwrap();
        let bindings = HashMap::from([(String::from("main"), Some(String::from("pond-example"))), (String::from("extra"), None)]);
        store.save_idle(json!({"mode":"off","text":""}), initial, None, Some(&bindings)).unwrap();
        let resized = store.save_panel_layout(json!({"version":2,"items":[{"id":"main","kind":"ecology","scalePercent":130}]})).unwrap();
        assert_eq!(resized["items"], json!([{"id":"main","kind":"ecology","scalePercent":130,"worldId":"pond-example"}]));
        assert_eq!(resized["idle"]["mode"], "off");
        fs::remove_file(root.join("layout.json")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}

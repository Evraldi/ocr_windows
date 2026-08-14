mod autostart;
mod capture;
mod clipboard;
mod decode_code;
mod history;
mod models;
mod ocr_text;
mod preprocess;

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use base64::Engine;
use image::{DynamicImage, ImageFormat};
use paddle_ocr_rs::ocr_lite::OcrLite;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::models::{AppInfo, HistoryEntry, Region, ScanResult};

pub struct AppState {
    history: Mutex<history::History>,
    ocr: Mutex<Option<OcrLite>>,
}

const SCAN_SHORTCUT: &str = "CommandOrControl+Shift+S";

fn overlay_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())
}

fn show_overlay(app: &AppHandle) -> Result<(), String> {
    let overlay = overlay_window(app)?;

    let target = app
        .cursor_position()
        .ok()
        .and_then(|pos| {
            app.available_monitors()
                .ok()
                .and_then(|mons| {
                    mons.into_iter().find(|m| {
                        let p = m.position();
                        let s = m.size();
                        (pos.x >= p.x as f64)
                            && (pos.y >= p.y as f64)
                            && (pos.x < p.x as f64 + s.width as f64)
                            && (pos.y < p.y as f64 + s.height as f64)
                    })
                })
        })
        .or_else(|| app.primary_monitor().ok().flatten());

    if let Some(mon) = target {
        overlay
            .set_position(tauri::Position::Physical(*mon.position()))
            .map_err(|e| e.to_string())?;
        overlay
            .set_size(tauri::Size::Physical(*mon.size()))
            .map_err(|e| e.to_string())?;
    }

    overlay
        .set_always_on_top(true)
        .map_err(|e| e.to_string())?;
    overlay.show().map_err(|e| e.to_string())?;
    overlay.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

fn hide_overlay(app: &AppHandle) -> Result<(), String> {
    overlay_window(app)?.hide().map_err(|e| e.to_string())
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn make_preview(img: &DynamicImage) -> Option<String> {
    let max_dim = 1600u32;
    let (w, h) = (img.width(), img.height());
    let scaled = if w.max(h) > max_dim {
        let scale = max_dim as f64 / f64::from(w.max(h));
        img.thumbnail(
            (f64::from(w) * scale).round().max(1.0) as u32,
            (f64::from(h) * scale).round().max(1.0) as u32,
        )
    } else {
        img.clone()
    };
    let mut buf = Vec::new();
    let cursor = Cursor::new(&mut buf);
    if scaled.write_to(cursor, ImageFormat::Png).is_err() {
        return None;
    }
    Some(base64::engine::general_purpose::STANDARD.encode(&buf))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn models_dir(app: &AppHandle) -> PathBuf {
    use tauri::path::BaseDirectory;
    if let Ok(p) = app.path().resolve("models", BaseDirectory::Resource) {
        if p.join("ch_PP-OCRv4_det_infer.onnx").exists() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("models")
}

fn load_ocr_engine(app: &AppHandle) -> Result<OcrLite, String> {
    let dir = models_dir(app);
    log::info!("[ocr] loading PaddleOCR engine from {}", dir.display());
    let det = std::fs::read(dir.join("ch_PP-OCRv4_det_infer.onnx"))
        .map_err(|e| format!("det model: {e}"))?;
    let cls = std::fs::read(dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx"))
        .map_err(|e| format!("cls model: {e}"))?;
    let rec = std::fs::read(dir.join("ch_PP-OCRv4_rec_infer.onnx"))
        .map_err(|e| format!("rec model: {e}"))?;
    let threads = std::thread::available_parallelism()
        .map(|n| n.get() / 2)
        .unwrap_or(2)
        .clamp(1, 8);
    ocr_text::init(&det, &cls, &rec, threads)
}

/// Merge multiple decoded codes into one result. When several distinct codes are
/// found in a region, all of them are copied (newline separated) instead of only
/// the first one being kept.
fn combine_codes(codes: &[(String, String)]) -> Option<(String, String)> {
    let mut seen = std::collections::HashSet::new();
    let mut contents = Vec::new();
    let mut formats = Vec::new();
    for (content, format) in codes {
        if seen.insert(content.clone()) {
            contents.push(content.clone());
            formats.push(format.clone());
        }
    }
    if contents.is_empty() {
        return None;
    }
    let content = contents.join("\n");
    let format = if formats.len() == 1 {
        formats.pop().expect("non-empty")
    } else {
        formats.join(" | ")
    };
    Some((content, format))
}

/// Decodes barcodes/QRs and runs OCR on `img`, copies the result to the
/// clipboard when something is found, stores it in history and broadcasts
/// the `scan-result` event.
fn process_scan(app: &AppHandle, img: DynamicImage) -> Result<ScanResult, String> {
    let preview = make_preview(&img);

    let codes = decode_code::decode(&img);
    let result = if let Some((content, format)) = combine_codes(&codes) {
        ScanResult {
            kind: "code".into(),
            content,
            format,
            preview: preview.clone(),
            timestamp: now(),
        }
    } else {
        let state = app.state::<AppState>();
        let mut ocr_guard = state.ocr.lock().map_err(|e| e.to_string())?;
        let mut ocr = ocr_guard.take();
        let ocr_out = match &mut ocr {
            Some(engine) => ocr_text::recognize_text(engine, &img),
            None => Err("OCR engine not initialized".to_string()),
        };
        *ocr_guard = ocr;

        match ocr_out {
            Ok(Some(text)) if !text.trim().is_empty() => ScanResult {
                kind: "text".into(),
                content: text.trim().to_string(),
                format: "text".into(),
                preview,
                timestamp: now(),
            },
            _ => ScanResult {
                kind: "none".into(),
                content: "No barcode/QR or text detected".into(),
                format: ocr_text::status(),
                preview,
                timestamp: now(),
            },
        }
    };

    if result.kind != "none" {
        let _ = clipboard::copy_text(&result.content);
        let state = app.state::<AppState>();
        let hist = state.history.lock().map_err(|e| e.to_string())?;
        let png = result
            .preview
            .as_ref()
            .and_then(|p| base64::engine::general_purpose::STANDARD.decode(p).ok());
        let _ = hist.insert(
            result.timestamp,
            &result.kind,
            &result.content,
            &result.format,
            png.as_deref(),
        );
    }

    let _ = app.emit("scan-result", result.clone());
    Ok(result)
}

#[tauri::command]
fn start_scan(app: AppHandle) -> Result<(), String> {
    show_overlay(&app)
}

#[tauri::command]
fn cancel_scan(app: AppHandle) -> Result<(), String> {
    hide_overlay(&app)
}

#[tauri::command]
async fn finish_scan(app: AppHandle, region: Region, dpr: f64) -> Result<ScanResult, String> {
    hide_overlay(&app)?;

    tauri::async_runtime::spawn_blocking(move || {
        // Let the compositor actually remove the overlay before grabbing the
        // screen, otherwise the screenshot can capture its dim layer.
        std::thread::sleep(Duration::from_millis(120));

        let img = capture::capture_region(&region, dpr).map_err(|e| e.to_string())?;
        process_scan(&app, img)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn scan_image(app: AppHandle, image_b64: String) -> Result<ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&image_b64)
            .map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes).map_err(|e| format!("unsupported image: {e}"))?;
        process_scan(&app, img)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn scan_clipboard_image(app: AppHandle) -> Result<Option<ScanResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(png) = clipboard::read_image_png().map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let img = image::load_from_memory(&png).map_err(|e| e.to_string())?;
        Ok(Some(process_scan(&app, img)?))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn delete_history_entry(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.history.lock().map_err(|e| e.to_string())?.delete(id)
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    autostart::set(enabled)
}

#[tauri::command]
fn get_autostart() -> Result<bool, String> {
    autostart::get()
}

#[tauri::command]
fn get_history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, String> {
    state.history.lock().map_err(|e| e.to_string())?.list(200)
}

#[tauri::command]
fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history.lock().map_err(|e| e.to_string())?.clear()
}

#[tauri::command]
fn copy_to_clipboard(content: String) -> Result<(), String> {
    clipboard::copy_text(&content)
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        shortcut: SCAN_SHORTCUT.to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new().build(),
        )
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let dir = app.path().app_data_dir()?;
            let hist = history::History::new(&dir)?;

            app.manage(AppState {
                history: Mutex::new(hist),
                ocr: Mutex::new(None),
            });

            for window_config in app.config().app.windows.iter() {
                tauri::WebviewWindowBuilder::from_config(app, window_config)?.build()?;
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                match load_ocr_engine(&handle) {
                    Ok(engine) => {
                        log::info!("[ocr] PaddleOCR engine ready ({})", ocr_text::status());
                        if let Some(state) = handle.try_state::<AppState>() {
                            if let Ok(mut guard) = state.ocr.lock() {
                                *guard = Some(engine);
                            }
                        }
                    }
                    Err(e) => log::error!("[ocr] failed to initialize engine: {e}"),
                }
            });

            let show = MenuItem::with_id(app, "show", "Open OCR Scan Tool", true, None::<&str>)?;
            let scan = MenuItem::with_id(app, "scan", "Scan Region", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &scan, &quit])?;

            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("OCR Scan Tool");
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    ..
                } = event
                {
                    show_main(tray.app_handle());
                }
            })
            .on_menu_event(|app, event| match event.id().as_ref() {
                "show" => show_main(app),
                "scan" => {
                    if let Err(e) = show_overlay(app) {
                        log::error!("failed to show overlay: {e}");
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;

            app.global_shortcut()
                .on_shortcut(SCAN_SHORTCUT, move |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Err(e) = show_overlay(app) {
                            log::error!("failed to show overlay: {e}");
                        }
                    }
                })
                .map_err(|e| e.to_string())?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // Keep the app alive in the tray; only a real quit exits.
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_scan,
            cancel_scan,
            finish_scan,
            scan_image,
            scan_clipboard_image,
            get_history,
            clear_history,
            delete_history_entry,
            copy_to_clipboard,
            app_info,
            set_autostart,
            get_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
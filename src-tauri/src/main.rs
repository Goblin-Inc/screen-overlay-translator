#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai_client;
mod os_strategy;

use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
struct TranslationResult {
    original: String,
    translated: String,
}

#[tauri::command]
async fn capture_and_translate(
    app: AppHandle,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    target_lang: String,
) -> Result<TranslationResult, String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    let ocr_res = tauri::async_runtime::spawn_blocking(move || {
        let text = os_strategy::capture_and_ocr(x, y, width, height, "eng+rus")?;
        if text.trim().is_empty() {
            return Ok(TranslationResult {
                original: String::new(),
                translated: "Текст в выбранной области не распознан. Попробуйте выделить крупнее.".to_string(),
            });
        }
        let translated = match ai_client::translate_text(&text, &target_lang) {
            Ok(t) => t,
            Err(e) => format!("⚠ {}", e),
        };
        Ok::<TranslationResult, String>(TranslationResult {
            original: text,
            translated,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }

    ocr_res
}

#[tauri::command]
fn retranslate_text(text: String, target_lang: String) -> Result<String, String> {
    ai_client::translate_text(&text, &target_lang)
}

#[tauri::command]
fn copy_clipboard(text: String) -> Result<(), String> {
    os_strategy::copy_to_system_clipboard(&text)
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Fix WebKitGTK Error 71 (Protocol error dispatching to Wayland display) on Hyprland / Wayland
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
        if std::env::var_os("GDK_BACKEND").is_none() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            capture_and_translate,
            retranslate_text,
            copy_clipboard,
            exit_app
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(false);
                let _ = window.set_always_on_top(true);
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

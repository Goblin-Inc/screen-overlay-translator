use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Делает снимок выделенной области экрана и распознаёт текст через Tesseract OCR.
pub fn capture_and_ocr(x: i32, y: i32, width: i32, height: i32, lang: &str) -> Result<String, String> {
    let w = width.max(16);
    let h = height.max(16);
    let left = x.max(0);
    let top = y.max(0);

    let img_path = "/tmp/sot_capture.png";
    let processed_path = "/tmp/sot_capture_prep.png";
    let _ = fs::remove_file(img_path);
    let _ = fs::remove_file(processed_path);

    // Небольшая пауза, чтобы окно выделения успело стать прозрачным перед снимком
    thread::sleep(Duration::from_millis(90));

    let captured = try_capture_region(left, top, w, h, img_path);
    if !captured || !std::path::Path::new(img_path).exists() {
        return Err("Не удалось захватить область экрана (проверьте maim / grim / spectacle)".to_string());
    }

    // Увеличиваем изображение в 2 раза и повышаем контраст через ImageMagick convert для точного OCR мелкого шрифта
    let mut ocr_input = img_path;
    if let Ok(status) = Command::new("convert")
        .args([
            img_path,
            "-resize",
            "200%",
            "-modulate",
            "100,0",
            "-contrast-stretch",
            "1%x1%",
            processed_path,
        ])
        .status()
    {
        if status.success() && std::path::Path::new(processed_path).exists() {
            ocr_input = processed_path;
        }
    }

    let ocr_lang = if lang.is_empty() { "eng+rus" } else { lang };
    let output = Command::new("tesseract")
        .args([ocr_input, "stdout", "-l", ocr_lang, "--psm", "6"])
        .output()
        .map_err(|e| format!("Ошибка запуска tesseract: {}", e))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Ошибка OCR: {}", err_str));
    }

    let raw_text = String::from_utf8_lossy(&output.stdout).to_string();
    let cleaned = clean_ocr_text(&raw_text);
    Ok(cleaned)
}

fn try_capture_region(x: i32, y: i32, w: i32, h: i32, out_path: &str) -> bool {
    let is_wayland = std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    // 1. Wayland: grim
    if is_wayland {
        let geom = format!("{},{} {}x{}", x, y, w, h);
        if let Ok(st) = Command::new("grim").args(["-g", &geom, out_path]).status() {
            if st.success() {
                return true;
            }
        }
    }

    // 2. X11 / KDE XWayland: maim (самый быстрый и точный)
    let maim_geom = format!("{}x{}+{}+{}", w, h, x, y);
    if let Ok(st) = Command::new("maim")
        .args(["-u", "-g", &maim_geom, out_path])
        .status()
    {
        if st.success() {
            return true;
        }
    }

    // 3. Fallback: ImageMagick import
    if let Ok(st) = Command::new("import")
        .args(["-window", "root", "-crop", &maim_geom, "+repage", out_path])
        .status()
    {
        if st.success() {
            return true;
        }
    }

    // 4. Fallback для KDE Wayland: spectacle в фоновом режиме + crop через convert
    let full_tmp = "/tmp/sot_full.png";
    if let Ok(st) = Command::new("spectacle")
        .args(["-b", "-n", "-f", "-o", full_tmp])
        .status()
    {
        if st.success() {
            if let Ok(cst) = Command::new("convert")
                .args([full_tmp, "-crop", &maim_geom, "+repage", out_path])
                .status()
            {
                let _ = fs::remove_file(full_tmp);
                if cst.success() {
                    return true;
                }
            }
        }
    }

    false
}

fn clean_ocr_text(raw: &str) -> String {
    let mut lines = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed);
        }
    }
    lines.join(" ")
}

/// Копирует текст в системный буфер обмена (X11 / Wayland / KDE Klipper)
pub fn copy_to_system_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::Stdio;

    for (cmd, args) in [
        ("xclip", vec!["-selection", "clipboard"]),
        ("xsel", vec!["--clipboard", "--input"]),
        ("wl-copy", vec![]),
    ] {
        if let Ok(mut child) = Command::new(cmd).args(&args).stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if let Ok(status) = child.wait() {
                if status.success() {
                    return Ok(());
                }
            }
        }
    }

    // KDE Klipper DBus fallback
    let _ = Command::new("qdbus6")
        .args([
            "org.kde.klipper",
            "/klipper",
            "setClipboardContents",
            text,
        ])
        .status();

    Ok(())
}

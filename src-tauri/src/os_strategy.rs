use std::fs;
use std::io::Write;
use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Creates a Command that hides console windows on Windows (prevents black cmd flashes).
#[allow(unused_mut)]
pub(crate) fn hidden_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Captures the selected screen region and recognizes text via OCR (cross-platform: Windows & Linux).
pub fn capture_and_ocr(x: i32, y: i32, width: i32, height: i32, lang: &str) -> Result<String, String> {
    let w = width.max(16);
    let h = height.max(16);
    let left = x.max(0);
    let top = y.max(0);

    let tmp_dir = std::env::temp_dir();
    let img_path = tmp_dir.join("sot_capture.png");
    let processed_path = tmp_dir.join("sot_capture_prep.png");
    let _ = fs::remove_file(&img_path);
    let _ = fs::remove_file(&processed_path);

    // Wait briefly so the overlay finishes hiding before screen capture
    thread::sleep(Duration::from_millis(120));

    let captured = try_capture_region(left, top, w, h, &img_path);
    if !captured || !img_path.exists() {
        #[cfg(target_os = "windows")]
        return Err("Не удалось захватить область экрана на Windows (ошибка PowerShell / GDI+).".to_string());
        #[cfg(not(target_os = "windows"))]
        return Err("Не удалось захватить область экрана (проверьте maim / grim / spectacle).".to_string());
    }

    let ocr_lang = if lang.is_empty() { "eng+rus" } else { lang };
    let raw_text = run_ocr(&img_path, &processed_path, ocr_lang)?;
    Ok(clean_ocr_text(&raw_text))
}

#[cfg(target_os = "windows")]
fn try_capture_region(x: i32, y: i32, w: i32, h: i32, out_path: &Path) -> bool {
    let escaped = out_path.to_string_lossy().replace('\'', "''");
    // DPI-aware GDI+ screen capture with 2x HighQualityBicubic upscale for crisp OCR
    let ps_script = format!(
        r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition '
using System.Runtime.InteropServices;
public static class SotDpi {{
    [DllImport("user32.dll")]
    public static extern bool SetProcessDPIAware();
}}'
[SotDpi]::SetProcessDPIAware() | Out-Null

$bmp = New-Object System.Drawing.Bitmap({w}, {h})
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen({x}, {y}, 0, 0, $bmp.Size)
$g.Dispose()

$w2 = {w} * 2
$h2 = {h} * 2
$scaled = New-Object System.Drawing.Bitmap($w2, $h2)
$g2 = [System.Drawing.Graphics]::FromImage($scaled)
$g2.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g2.DrawImage($bmp, 0, 0, $w2, $h2)
$g2.Dispose()
$bmp.Dispose()

$scaled.Save('{out}', [System.Drawing.Imaging.ImageFormat]::Png)
$scaled.Dispose()
"#,
        x = x,
        y = y,
        w = w,
        h = h,
        out = escaped
    );

    if let Ok(status) = hidden_cmd("powershell")
        .args(["-NoProfile", "-NonInteractive", "-STA", "-Command", &ps_script])
        .status()
    {
        return status.success() && out_path.exists();
    }
    false
}

#[cfg(target_os = "windows")]
fn find_windows_tesseract() -> Option<PathBuf> {
    // 1. Check tesseract next to the application binary (portable bundle)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            for rel in ["tesseract\\tesseract.exe", "tesseract.exe"] {
                let candidate = dir.join(rel);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    // 2. Check standard Windows installation paths
    for p in [
        r"C:\Program Files\Tesseract-OCR\tesseract.exe",
        r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
    ] {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    // 3. Check PATH
    if let Ok(st) = hidden_cmd("tesseract").arg("--version").status() {
        if st.success() {
            return Some(PathBuf::from("tesseract"));
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn run_windows_builtin_ocr(img_path: &Path) -> Result<String, String> {
    let escaped = img_path.to_string_lossy().replace('\'', "''");
    // Built-in Windows 10/11 WinRT OCR (requires zero external software installed!)
    let ps_script = format!(
        r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Globalization.Language, Windows.Foundation, ContentType = WindowsRuntime]

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
}})[0]

function Await($WinRtTask, $ResultType) {{
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}}

$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync('{img}')) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])

$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) {{
    $lang = New-Object Windows.Globalization.Language('en-US')
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($lang)
}}
if ($null -eq $engine) {{
    throw "Windows OCR language pack is unavailable"
}}

$res = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
$stream.Dispose()
Write-Output $res.Text
"#,
        img = escaped
    );

    let output = hidden_cmd("powershell")
        .args(["-NoProfile", "-NonInteractive", "-STA", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Ошибка запуска Windows OCR: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Windows OCR не сработал ({}). Установите Tesseract-OCR или языковой пакет Windows.",
            err
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(target_os = "windows")]
fn run_ocr(img_path: &Path, _processed_path: &Path, ocr_lang: &str) -> Result<String, String> {
    // 1. Prefer Tesseract if installed (supports eng+rus simultaneously)
    if let Some(tess_bin) = find_windows_tesseract() {
        let img_str = img_path.to_string_lossy().to_string();
        let bin_str = tess_bin.to_string_lossy().to_string();
        if let Ok(output) = hidden_cmd(&bin_str)
            .args([&img_str, "stdout", "-l", ocr_lang, "--psm", "6"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }
        }
    }

    // 2. Zero-install fallback: native Windows 10/11 OCR
    run_windows_builtin_ocr(img_path)
}

#[cfg(not(target_os = "windows"))]
fn try_capture_region(x: i32, y: i32, w: i32, h: i32, out_path: &Path) -> bool {
    let out_str = out_path.to_string_lossy().to_string();
    let is_wayland = std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    // 1. Wayland: grim
    if is_wayland {
        let geom = format!("{},{} {}x{}", x, y, w, h);
        if let Ok(st) = Command::new("grim").args(["-g", &geom, &out_str]).status() {
            if st.success() {
                return true;
            }
        }
    }

    // 2. X11 / XWayland: maim
    let maim_geom = format!("{}x{}+{}+{}", w, h, x, y);
    if let Ok(st) = Command::new("maim")
        .args(["-u", "-g", &maim_geom, &out_str])
        .status()
    {
        if st.success() {
            return true;
        }
    }

    // 3. Fallback: ImageMagick import
    if let Ok(st) = Command::new("import")
        .args(["-window", "root", "-crop", &maim_geom, "+repage", &out_str])
        .status()
    {
        if st.success() {
            return true;
        }
    }

    // 4. Fallback for KDE Wayland: spectacle + crop via convert
    let full_tmp = std::env::temp_dir().join("sot_full.png");
    let full_str = full_tmp.to_string_lossy().to_string();
    if let Ok(st) = Command::new("spectacle")
        .args(["-b", "-n", "-f", "-o", &full_str])
        .status()
    {
        if st.success() {
            if let Ok(cst) = Command::new("convert")
                .args([&full_str, "-crop", &maim_geom, "+repage", &out_str])
                .status()
            {
                let _ = fs::remove_file(&full_tmp);
                if cst.success() {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(not(target_os = "windows"))]
fn run_ocr(img_path: &Path, processed_path: &Path, ocr_lang: &str) -> Result<String, String> {
    let img_str = img_path.to_string_lossy().to_string();
    let proc_str = processed_path.to_string_lossy().to_string();

    // Upscale 2x & boost contrast with ImageMagick on Linux for crisp OCR on small text
    let mut ocr_input = img_str.clone();
    if let Ok(status) = Command::new("convert")
        .args([
            &img_str,
            "-resize",
            "200%",
            "-modulate",
            "100,0",
            "-contrast-stretch",
            "1%x1%",
            &proc_str,
        ])
        .status()
    {
        if status.success() && processed_path.exists() {
            ocr_input = proc_str;
        }
    }

    let output = Command::new("tesseract")
        .args([&ocr_input, "stdout", "-l", ocr_lang, "--psm", "6"])
        .output()
        .map_err(|e| format!("Ошибка запуска tesseract: {}", e))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Ошибка OCR: {}", err_str));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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

/// Copies UTF-8 text to the system clipboard on both Windows and Linux.
pub fn copy_to_system_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let ps = "[Console]::InputEncoding = [System.Text.Encoding]::UTF8; $t = [Console]::In.ReadToEnd(); Set-Clipboard -Value $t";
        if let Ok(mut child) = hidden_cmd("powershell")
            .args(["-NoProfile", "-NonInteractive", "-STA", "-Command", ps])
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if let Ok(st) = child.wait() {
                if st.success() {
                    return Ok(());
                }
            }
        }
        return Err("Не удалось скопировать в буфер обмена Windows".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        for (cmd, args) in [
            ("wl-copy", vec![]),
            ("xclip", vec!["-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--input"]),
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
}

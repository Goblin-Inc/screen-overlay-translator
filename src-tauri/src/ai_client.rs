use serde_json::Value;
use std::io::Write;
use std::process::Stdio;

use crate::os_strategy::hidden_cmd;

#[cfg(target_os = "windows")]
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

#[cfg(not(target_os = "windows"))]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

#[allow(unused_mut)]
fn curl_cmd() -> std::process::Command {
    let mut cmd = hidden_cmd("curl");
    #[cfg(target_os = "windows")]
    {
        // Avoid Windows Schannel CRL revocation failures on school / corporate networks
        cmd.arg("--ssl-no-revoke");
    }
    cmd
}

/// Автоматически определяет направление перевода (EN -> RU или RU -> EN) и переводит текст
/// через каскад из 4 резервных провайдеров.
pub fn translate_text(text: &str, target_lang: &str) -> Result<String, String> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return Ok("Текст на выделенном участке не обнаружен.".to_string());
    }

    let is_cyr = has_mostly_cyrillic(cleaned);
    let resolved_target = if target_lang.is_empty() || target_lang == "auto" {
        if is_cyr { "en" } else { "ru" }
    } else {
        target_lang
    };

    let source_lang = if is_cyr { "ru" } else { "en" };

    // 1. Google Chrome Dictionary / Translate API (clients5)
    if let Ok(translated) = call_clients5_google(cleaned, resolved_target) {
        if !translated.trim().is_empty() {
            return Ok(translated);
        }
    }

    // 2. Google Translate GTX API
    if let Ok(translated) = call_google_gtx(cleaned, resolved_target) {
        if !translated.trim().is_empty() {
            return Ok(translated);
        }
    }

    // 3. MyMemory Translation API
    if let Ok(translated) = call_mymemory(cleaned, source_lang, resolved_target) {
        if !translated.trim().is_empty() {
            return Ok(translated);
        }
    }

    // 4. Context-aware AI translator fallback (Pollinations OpenAI endpoint)
    if let Ok(ai_translated) = call_ai_translator(cleaned, resolved_target) {
        if !ai_translated.trim().is_empty() {
            return Ok(ai_translated);
        }
    }

    Err("Не удалось подключиться к сервису перевода.".to_string())
}

fn has_mostly_cyrillic(text: &str) -> bool {
    let mut cyr = 0usize;
    let mut lat = 0usize;
    for ch in text.chars() {
        if ('\u{0400}'..='\u{04FF}').contains(&ch) {
            cyr += 1;
        } else if ch.is_ascii_alphabetic() {
            lat += 1;
        }
    }
    cyr > lat
}

fn call_clients5_google(text: &str, target_lang: &str) -> Result<String, String> {
    let output = curl_cmd()
        .args([
            "-sS",
            "--max-time",
            "8",
            "-A",
            USER_AGENT,
            "-G",
            "https://clients5.google.com/translate_a/t",
            "--data-urlencode",
            "client=dict-chrome-ex",
            "--data-urlencode",
            "sl=auto",
            "--data-urlencode",
            &format!("tl={}", target_lang),
            "--data-urlencode",
            &format!("q={}", text),
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("clients5 curl failed".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    if let Some(arr) = parsed.as_array() {
        let mut result = String::new();
        for item in arr {
            if let Some(s) = item.as_str() {
                result.push_str(s);
            } else if let Some(inner) = item.as_array() {
                if let Some(s) = inner.first().and_then(|v| v.as_str()) {
                    result.push_str(s);
                }
            }
        }
        if !result.trim().is_empty() {
            return Ok(result.trim().to_string());
        }
    }

    Err("empty clients5 response".to_string())
}

fn call_google_gtx(text: &str, target_lang: &str) -> Result<String, String> {
    let output = curl_cmd()
        .args([
            "-sS",
            "--max-time",
            "8",
            "-A",
            USER_AGENT,
            "-G",
            "https://translate.googleapis.com/translate_a/single",
            "--data-urlencode",
            "client=gtx",
            "--data-urlencode",
            "sl=auto",
            "--data-urlencode",
            &format!("tl={}", target_lang),
            "--data-urlencode",
            "dt=t",
            "--data-urlencode",
            &format!("q={}", text),
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("curl gtx failed".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let mut result = String::new();
    if let Some(sentences) = parsed.get(0).and_then(|v| v.as_array()) {
        for item in sentences {
            if let Some(part) = item.get(0).and_then(|v| v.as_str()) {
                result.push_str(part);
            }
        }
    }

    if result.trim().is_empty() {
        Err("empty gtx translation".to_string())
    } else {
        Ok(result.trim().to_string())
    }
}

fn call_mymemory(text: &str, source_lang: &str, target_lang: &str) -> Result<String, String> {
    let src = if source_lang == target_lang {
        if target_lang == "ru" { "en" } else { "ru" }
    } else {
        source_lang
    };
    let langpair = format!("{}|{}", src, target_lang);
    let clipped: String = text.chars().take(480).collect();

    let output = curl_cmd()
        .args([
            "-sS",
            "--max-time",
            "8",
            "-A",
            USER_AGENT,
            "-G",
            "https://api.mymemory.translated.net/get",
            "--data-urlencode",
            &format!("q={}", clipped),
            "--data-urlencode",
            &format!("langpair={}", langpair),
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("mymemory failed".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let translated = parsed["responseData"]["translatedText"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if translated.is_empty() {
        Err("empty mymemory".to_string())
    } else {
        Ok(translated)
    }
}

fn call_ai_translator(text: &str, target_lang: &str) -> Result<String, String> {
    let lang_name = match target_lang {
        "en" => "English",
        "de" => "German",
        "ja" => "Japanese",
        _ => "Russian",
    };

    let payload = serde_json::json!({
        "model": "openai",
        "messages": [
            {
                "role": "system",
                "content": format!(
                    "You are an accurate screen OCR translator. Translate the user's OCR text into natural {}. Output ONLY the translated text without quotes or explanations.",
                    lang_name
                )
            },
            {
                "role": "user",
                "content": text
            }
        ],
        "temperature": 0.2
    });

    // Pass UTF-8 JSON body via stdin (--data-binary @-) so Windows cmd-line escaping never mangles Cyrillic
    let mut child = curl_cmd()
        .args([
            "-sS",
            "--max-time",
            "12",
            "-A",
            USER_AGENT,
            "-X",
            "POST",
            "https://text.pollinations.ai/openai",
            "-H",
            "Content-Type: application/json; charset=utf-8",
            "--data-binary",
            "@-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload.to_string().as_bytes());
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let body = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let content = parsed["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        Err("empty AI translation".to_string())
    } else {
        Ok(content)
    }
}

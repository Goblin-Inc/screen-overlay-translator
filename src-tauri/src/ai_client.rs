use serde_json::Value;
use std::process::Command;

/// Автоматически определяет направление перевода (EN -> RU или RU -> EN) и переводит текст.
pub fn translate_text(text: &str, target_lang: &str) -> Result<String, String> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return Ok("Текст на выделенном участке не обнаружен.".to_string());
    }

    let resolved_target = if target_lang.is_empty() || target_lang == "auto" {
        if has_mostly_cyrillic(cleaned) {
            "en"
        } else {
            "ru"
        }
    } else {
        target_lang
    };

    // 1. Быстрый запрос к Google Translate GTX API через curl
    if let Ok(translated) = call_google_gtx(cleaned, resolved_target) {
        if !translated.trim().is_empty() {
            return Ok(translated);
        }
    }

    // 2. Fallback: Контекстный AI-переводчик (Pollinations OpenAI endpoint)
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

fn call_google_gtx(text: &str, target_lang: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "8",
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

    if result.is_empty() {
        Err("empty translation".to_string())
    } else {
        Ok(result)
    }
}

fn call_ai_translator(text: &str, target_lang: &str) -> Result<String, String> {
    let lang_name = if target_lang == "en" {
        "English"
    } else {
        "Russian"
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

    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "12",
            "-X",
            "POST",
            "https://text.pollinations.ai/openai",
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload.to_string(),
        ])
        .output()
        .map_err(|e| e.to_string())?;

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

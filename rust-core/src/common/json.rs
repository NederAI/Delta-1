//! Extremely small JSON helpers tailored to the architecture scaffolding.
//!
//! The goal is to avoid pulling additional dependencies while still being able
//! to inspect a handful of keys inside configuration and request payloads.
//! These helpers are **not** a general purpose parser – they assume well-formed
//! JSON with double quoted keys and primitive values.

/// Escape a string so it can be embedded into JSON output.
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn locate_key<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{}\"", key);
    let idx = source.find(&pattern)?;
    Some(&source[idx + pattern.len()..])
}

/// Extract a boolean value for the provided key.
pub fn extract_bool(source: &str, key: &str) -> Option<bool> {
    let after = locate_key(source, key)?;
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Extract a floating point number for the provided key.
pub fn extract_number(source: &str, key: &str) -> Option<f32> {
    let after = locate_key(source, key)?;
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    let mut len = 0;
    for ch in rest.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E') {
            len += ch.len_utf8();
        } else {
            break;
        }
    }
    if len == 0 {
        return None;
    }
    rest[..len].parse().ok()
}

/// Extract a string value (without surrounding quotes) for the provided key.
pub fn extract_string(source: &str, key: &str) -> Option<String> {
    let after = locate_key(source, key)?;
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    out.push(match next {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    });
                }
            }
            '"' => return Some(out),
            other => out.push(other),
        }
    }
    None
}

/// Extract a JSON object (including braces) for the provided key.
pub fn extract_object<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let after = locate_key(source, key)?;
    let brace = after.find('{')?;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let bytes = after[brace..].as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        let ch = b as char;
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' | '[' if !in_string => depth += 1,
            '}' | ']' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&after[brace..=brace + idx]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Collect top-level keys from a JSON object.
pub fn top_level_keys(source: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut current = String::new();
    let mut reading_key = false;

    for ch in source.chars() {
        if escape {
            if in_string && reading_key {
                current.push(ch);
            }
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape = true,
            '"' => {
                if in_string {
                    if reading_key && depth == 1 {
                        keys.push(current.clone());
                    }
                    in_string = false;
                    current.clear();
                } else if depth == 1 {
                    reading_key = true;
                    in_string = true;
                }
            }
            '{' | '[' if !in_string => {
                depth += 1;
                if depth == 1 {
                    reading_key = false;
                }
            }
            '}' | ']' if !in_string => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ':' if !in_string && depth == 1 => {
                reading_key = false;
            }
            ',' if !in_string && depth == 1 => {
                reading_key = false;
            }
            _ => {
                if in_string && reading_key {
                    current.push(ch);
                }
            }
        }
    }

    keys
}

/// Build a JSON array from already escaped string elements.
pub fn build_string_array(items: &[String]) -> String {
    let mut out = String::from("[");
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&escape(item));
        out.push('"');
    }
    out.push(']');
    out
}

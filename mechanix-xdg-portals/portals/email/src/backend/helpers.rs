use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone)]
pub struct EmailRequestData {
    pub address: Option<String>,
    pub addresses: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub attachments: Vec<String>,
}

/// Parses the D-Bus options map into an EmailRequestData structure.
pub fn parse_email_data(options: &HashMap<String, OwnedValue>) -> EmailRequestData {
    EmailRequestData {
        address: get_string(options, "address"),
        addresses: get_string_list(options, "addresses"),
        cc: get_string_list(options, "cc"),
        bcc: get_string_list(options, "bcc"),
        subject: get_string(options, "subject"),
        body: get_string(options, "body"),
        attachments: get_string_list(options, "attachments"),
    }
}

/// Extracts a string value from an OwnedValue options map.
fn get_string(options: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    options.get(key).and_then(|v| {
        if let zbus::zvariant::Value::Str(s) = &**v {
            Some(s.to_string())
        } else {
            None
        }
    })
}

/// Extracts a Vec<String> from an OwnedValue options map.
fn get_string_list(options: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    options
        .get(key)
        .and_then(|v| {
            if let zbus::zvariant::Value::Array(arr) = &**v {
                let strings: Vec<String> = arr
                    .iter()
                    .filter_map(|item| {
                        if let zbus::zvariant::Value::Str(s) = item {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                Some(strings)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Encodes a string for a URL query parameter conforming to RFC 3986.
///
/// # Examples
///
/// ```
/// // Encodes spaces and special characters:
/// // "My Report.pdf" -> "My%20Report.pdf"
/// ```
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match b {
            // Keep RFC 3986 unreserved characters as-is (e.g., 'a', 'Z', '9', '-', '.')
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char);
            }
            // Percent-encode all other characters into %XX hex format (e.g., ' ' -> %20, '&' -> %26, non-ASCII)
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

/// Decodes a percent-encoded string.
///
/// # Examples
///
/// ```
/// // Decodes spaces and special characters:
/// // "My%20Report.pdf" -> "My Report.pdf"
/// ```
pub fn percent_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let mut input = s.as_bytes().iter();
    while let Some(&b) = input.next() {
        if b == b'%' {
            let mut matched = false;
            let next_h1 = input.clone().next();
            let next_h2 = input.clone().skip(1).next();
            if let (Some(&h1), Some(&h2)) = (next_h1, next_h2) {
                if let Some(decoded) = hex_decode(h1 as char, h2 as char) {
                    bytes.push(decoded);
                    input.next(); // Consume h1
                    input.next(); // Consume h2
                    matched = true;
                }
            }
            if !matched {
                bytes.push(b'%');
            }
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| s.to_string())
}

fn hex_decode(c1: char, c2: char) -> Option<u8> {
    let d1 = c1.to_digit(16)?;
    let d2 = c2.to_digit(16)?;
    Some((d1 << 4 | d2) as u8)
}

/// Converts a local filesystem path into a valid, percent-encoded file:// URI.
pub fn to_file_uri(path: &str) -> String {
    let encoded_segments: Vec<String> = path.split('/').map(percent_encode).collect();
    format!("file://{}", encoded_segments.join("/"))
}

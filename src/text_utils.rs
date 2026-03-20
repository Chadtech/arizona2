pub fn normalize_message_content(content: &str) -> String {
    let trimmed = content.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if first == b'"' && last == b'"' {
            return trimmed[1..bytes.len() - 1].trim().to_string();
        }
    }

    trimmed.to_string()
}

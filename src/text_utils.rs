pub fn normalize_message_content(content: &str) -> &str {
    let bytes = content.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if first == b'"' && last == b'"' {
            return &content[1..bytes.len() - 1];
        }
    }

    content
}

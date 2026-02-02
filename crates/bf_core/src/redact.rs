pub fn redact_id(value: &str) -> String {
    let len = value.chars().count();
    if len <= 8 {
        return "***".to_string();
    }
    let prefix: String = value.chars().take(4).collect();
    let suffix: String = value.chars().skip(len.saturating_sub(4)).collect();
    format!("{}***{}", prefix, suffix)
}

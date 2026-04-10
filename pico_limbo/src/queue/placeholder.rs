pub fn apply_placeholders(
    text: &str,
    position: usize,
    total: usize,
    player: &str,
    push_interval_secs: u64,
    push_count: usize,
) -> String {
    let eta = if push_count == 0 {
        0
    } else {
        ((position.saturating_sub(1)) / push_count) as u64 * push_interval_secs
    };
    text.replace("{position}", &position.to_string())
        .replace("{total}", &total.to_string())
        .replace("{player}", player)
        .replace("{eta}", &eta.to_string())
}

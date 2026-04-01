/// Load a theme's CSS by name, falling back to "minimal".
pub fn load_theme(name: &str) -> String {
    match name {
        "dark" => include_str!("../../themes/dark.css").to_string(),
        _ => include_str!("../../themes/minimal.css").to_string(),
    }
}

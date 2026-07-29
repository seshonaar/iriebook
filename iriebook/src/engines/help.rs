//! Authoring guide rendering
//!
//! Serves the bundled `assets/authoring-guide.html` as a self-contained HTML
//! document for display in the user's default browser. Magic numbers that the
//! prose references (scene-break threshold, fuzzy-prefix edit distance, ...) are
//! interpolated from the engine's own `pub const`s so the guide can never drift
//! out of sync with the rules it describes.

use crate::engines::text_processing::markdown_transform::{
    CHUNKY_PARAGRAPH_THRESHOLD, FUZZY_PREFIX_MAX_EDIT_DISTANCE,
};

/// The raw bundled HTML, compiled into the binary.
const AUTHORING_GUIDE_HTML: &str = include_str!("../../assets/authoring-guide.html");

/// Token → value pairs surfaced from code so the guide never drifts.
///
/// Add an entry here whenever the prose references a tunable constant. The token
/// must appear in `assets/authoring-guide.html` as `{{TOKEN}}`.
fn interpolation_map() -> Vec<(&'static str, String)> {
    vec![
        (
            "CHUNKY_PARAGRAPH_THRESHOLD",
            CHUNKY_PARAGRAPH_THRESHOLD.to_string(),
        ),
        (
            "FUZZY_PREFIX_MAX_EDIT_DISTANCE",
            FUZZY_PREFIX_MAX_EDIT_DISTANCE.to_string(),
        ),
    ]
}

/// Replace every `{{TOKEN}}` placeholder in `html` with its code-derived value.
fn interpolate(html: &str) -> String {
    let mut out = html.to_string();
    for (token, value) in interpolation_map() {
        let placeholder = format!("{{{{{token}}}}}");
        out = out.replace(&placeholder, &value);
    }
    out
}

/// The fully interpolated authoring guide as self-contained HTML.
pub fn authoring_guide_html() -> String {
    interpolate(AUTHORING_GUIDE_HTML)
}

/// The authoring guide as a self-contained HTML document (for the browser).
pub fn render_authoring_guide() -> String {
    authoring_guide_html()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_replaces_known_tokens() {
        let input = "Scene breaks need {{CHUNKY_PARAGRAPH_THRESHOLD}} lines, typos allow \
             edit distance {{FUZZY_PREFIX_MAX_EDIT_DISTANCE}}.";
        let out = interpolate(input);
        assert!(!out.contains("{{"), "unresolved token left behind: {out}");
        assert!(out.contains("need 10 lines"));
        assert!(out.contains("edit distance 2."));
    }

    #[test]
    fn render_authoring_guide_serves_complete_html() {
        let html = render_authoring_guide();
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>IrieBook Authoring Guide</title>"));
        assert!(html.contains("<body"));
    }

    #[test]
    fn bundled_guide_contains_rendered_examples() {
        let html = render_authoring_guide();
        assert!(html.contains("rendering-example"));
        assert!(html.contains("<p class=\"ebook-scene-break\">❦</p>"));
        assert!(html.contains("<section class=\"ebook-page dedication-page\""));
    }

    #[test]
    fn bundled_guide_has_no_unresolved_tokens() {
        // Every token we promise to interpolate must actually be resolved in the
        // rendered guide. Lowercase {{mustache}} placeholders that document the
        // previous-books template syntax are legitimate prose, not our tokens.
        let rendered = authoring_guide_html();
        for (token, _) in interpolation_map() {
            let placeholder = format!("{{{{{token}}}}}");
            assert!(
                !rendered.contains(&placeholder),
                "authoring-guide.html contains unresolved token {placeholder}"
            );
        }
    }

    #[test]
    fn bundled_guide_loads_without_panic() {
        let html = render_authoring_guide();
        assert!(html.len() > 200);
    }
}

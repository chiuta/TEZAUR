//! Static accessibility checks of the web UI (`tezaur.html` and the PWA copy built from it): accessible names,
//! keyboard access to citation markers, a live status region, selected-state of the mode tabs, contrast ratios
//! computed from the CSS colour variables in both themes, target size and the document language.
//! Checks that need a real browser (reflow at 320 px, screen readers) are not here; see INTREBARI.md.

#[cfg(test)]
mod tests {
    const PAGE: &str = include_str!("../tezaur.html");
    const PWA: &str = include_str!("../pwa/index.html");

    fn tags<'a>(html: &'a str, names: &[&str]) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut i = 0;
        while let Some(p) = html[i..].find('<') {
            let s = i + p;
            let rest = &html[s + 1..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if names.contains(&name.as_str())
                && let Some(e) = rest.find('>')
            {
                out.push(&html[s..s + 1 + e + 1]);
            }
            i = s + 1;
        }
        out
    }

    fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
        let pat = format!(" {name}=\"");
        let s = tag.find(&pat)? + pat.len();
        Some(&tag[s..s + tag[s..].find('"')?])
    }

    /// Everything between `<body` and `<script>`: the static markup, without the CSS and the code.
    fn body(html: &str) -> &str {
        let b = html.find("<body").unwrap();
        &html[b..b + html[b..].find("<script>").unwrap()]
    }

    #[test]
    fn every_field_has_an_accessible_name() {
        for page in [PAGE, PWA] {
            let b = body(page);
            let labels_for: Vec<&str> = tags(b, &["label"])
                .iter()
                .filter_map(|t| attr(t, "for"))
                .collect();
            let mut checked = 0;
            for t in tags(b, &["input", "select", "textarea"]) {
                if t.contains(" hidden") || attr(t, "type") == Some("file") {
                    continue; // hidden file pickers are opened through visible buttons
                }
                checked += 1;
                let named = attr(t, "aria-label").is_some_and(|v| !v.trim().is_empty())
                    || attr(t, "id").is_some_and(|id| labels_for.contains(&id))
                    || attr(t, "type") == Some("checkbox"); // these sit inside their <label>
                assert!(named, "field without a name: {t}");
            }
            assert!(checked >= 15);
            // checkboxes really are wrapped by a label
            for t in tags(b, &["input"])
                .iter()
                .filter(|t| attr(t, "type") == Some("checkbox"))
            {
                let at = b.find(*t).unwrap();
                let before = &b[..at];
                assert!(
                    before.rfind("<label").unwrap() > before.rfind("</label>").unwrap_or(0)
                        || before
                            .rfind("<label")
                            .is_some_and(|l| before.rfind("</label>").is_none_or(|c| l > c)),
                    "{t}"
                );
            }
        }
    }

    #[test]
    fn icon_only_buttons_are_named() {
        for id in ["mic", "mic2", "speak"] {
            let t = tags(body(PAGE), &["button"])
                .into_iter()
                .find(|t| attr(t, "id") == Some(id))
                .unwrap();
            assert!(attr(t, "aria-label").is_some(), "{t}");
        }
        // generated delete buttons and document checkboxes carry names
        assert!(PAGE.contains("class=\"x\" title=\"Șterge\" aria-label="));
        assert!(PAGE.contains("<input type=\"checkbox\" aria-label=\"${esc(d.title)}\""));
    }

    #[test]
    fn citation_markers_are_keyboard_operable() {
        assert!(PAGE.contains("data-n=\"${x}\" role=\"button\" tabindex=\"0\""));
        // Enter and Space run the same function as the click
        assert!(PAGE.contains("$('out').addEventListener('click',openCite)"));
        assert!(PAGE.contains("e.key==='Enter'||e.key===' '"));
        assert!(PAGE.contains("openCite(e)"));
    }

    #[test]
    fn status_regions_exist_before_they_are_updated() {
        for (id, role) in [
            ("status", "status"),
            ("warns", "status"),
            ("announce", "status"),
            ("bibStatus", "status"),
        ] {
            let t = tags(body(PAGE), &["div"])
                .into_iter()
                .find(|t| attr(t, "id") == Some(id))
                .unwrap_or_else(|| panic!("{id}"));
            assert_eq!(attr(t, "role"), Some(role), "{t}");
        }
        // progressive output is not a live region: only start and end are announced
        let out = tags(body(PAGE), &["div"])
            .into_iter()
            .find(|t| attr(t, "id") == Some("out"))
            .unwrap();
        assert!(attr(out, "aria-live").is_none() && attr(out, "role").is_none());
        assert!(PAGE.contains("$('announce').textContent=b?"));
    }

    #[test]
    fn mode_tabs_expose_the_selected_one() {
        let t = tags(body(PAGE), &["div"])
            .into_iter()
            .find(|t| attr(t, "id") == Some("tabs"))
            .unwrap();
        assert_eq!(attr(t, "role"), Some("group"));
        assert!(attr(t, "aria-label").is_some());
        assert!(PAGE.contains("b.setAttribute('aria-pressed',b.dataset.m===m?'true':'false')"));
        // not colour alone: the active tab also changes weight and gets an underline bar
        assert!(
            PAGE.contains(".tabs button.on{") && PAGE.contains("font-weight:700;box-shadow:inset")
        );
    }

    #[test]
    fn language_attribute_follows_the_ui_language() {
        assert!(PAGE.contains("<html lang=\"ro\">"));
        assert!(PAGE.contains("function applyLang(){document.documentElement.lang=UL;"));
    }

    #[test]
    fn delete_button_target_and_focus_style() {
        assert!(PAGE.contains(".x{") && PAGE.contains("min-width:24px;min-height:24px"));
        assert!(PAGE.contains(":focus-visible{outline:2px solid var(--acc)"));
        assert!(!PAGE.contains("outline:none") && !PAGE.contains("outline:0"));
    }

    // ---- contrast (WCAG relative luminance, sRGB threshold 0.04045)

    fn rgb(h: &str) -> [f64; 3] {
        let h = h.trim_start_matches('#');
        let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f64 / 255.0;
        [v(0), v(2), v(4)]
    }
    fn lum(h: &str) -> f64 {
        let c = rgb(h).map(|x| {
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    }
    fn ratio(a: &str, b: &str) -> f64 {
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }
    fn vars(block: &str) -> std::collections::HashMap<String, String> {
        block
            .split(';')
            .filter_map(|d| {
                let d = d.trim().trim_start_matches('{');
                let (k, v) = d.split_once(':')?;
                k.trim()
                    .strip_prefix("--")
                    .map(|k| (k.to_string(), v.trim().trim_end_matches('}').to_string()))
            })
            .collect()
    }
    fn themes() -> [std::collections::HashMap<String, String>; 2] {
        let l = PAGE.find(":root{").unwrap();
        let light = vars(&PAGE[l + 5..l + PAGE[l..].find('}').unwrap() + 1]);
        let d0 = PAGE.find("prefers-color-scheme:dark){:root{").unwrap()
            + "prefers-color-scheme:dark){:root".len();
        let dark = vars(&PAGE[d0..d0 + PAGE[d0..].find('}').unwrap() + 1]);
        [light, dark]
    }

    #[test]
    fn contrast_known_value() {
        assert!((ratio("#000000", "#ffffff") - 21.0).abs() < 1e-9);
        assert!((ratio("#23211c", "#f6f4ef") - 14.63).abs() < 0.01);
    }

    #[test]
    fn text_contrast_at_least_4_5_in_both_themes() {
        for t in themes() {
            for fg in ["ink", "mute", "bad", "ok", "warn", "acc"] {
                for bg in ["bg", "panel"] {
                    let r = ratio(&t[fg], &t[bg]);
                    assert!(r >= 4.5, "{fg} on {bg}: {r:.2}");
                }
            }
            let r = ratio(&t["panel"], &t["acc"]); // button text
            assert!(r >= 4.5, "button text {r:.2}");
        }
    }

    #[test]
    fn field_borders_and_focus_ring_at_least_3_to_1() {
        for t in themes() {
            for bg in ["bg", "panel"] {
                let r = ratio(&t["edge"], &t[bg]);
                assert!(r >= 3.0, "field border on {bg}: {r:.2}");
                assert!(ratio(&t["acc"], &t[bg]) >= 3.0, "focus ring on {bg}");
            }
        }
        // inputs actually use the stronger edge colour
        assert!(PAGE.contains("border:1px solid var(--edge);border-radius:6px"));
    }

    #[test]
    fn pwa_copy_is_in_step() {
        assert!(PWA.contains("aria-pressed") && PWA.contains("id=\"announce\""));
    }
}

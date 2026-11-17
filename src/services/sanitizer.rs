use std::sync::LazyLock;

use regex::{Captures, Regex};
use url::Url;

static MD_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").expect("valid markdown link regex"));

static HREF_DOUBLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]+)""#).expect("valid href double-quote regex"));

static HREF_SINGLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"href='([^']+)'").expect("valid href single-quote regex"));

fn is_disallowed_url(raw_url: &str, normalized_domains: &[String]) -> bool {
    let Ok(parsed) = Url::parse(raw_url.trim()) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let host_lower = host.to_lowercase();
    normalized_domains.iter().any(|domain| {
        host_lower == *domain || host_lower.ends_with(&format!(".{domain}"))
    })
}

/// Strips URLs matching `disallowed_domains` (exact or subdomain match, case-insensitive)
/// from Markdown links `[text](url)` and HTML `href="..."` / `href='...'` attributes.
pub fn strip_disallowed_urls(text: &str, disallowed_domains: &[String]) -> String {
    let normalized_domains: Vec<String> = disallowed_domains
        .iter()
        .map(|d| d.trim().to_lowercase())
        .filter(|d| !d.is_empty())
        .collect();

    if normalized_domains.is_empty() {
        return text.to_string();
    }

    let step1 = MD_LINK_RE.replace_all(text, |caps: &Captures| {
        let label = &caps[1];
        let raw_url = &caps[2];
        if is_disallowed_url(raw_url, &normalized_domains) {
            format!("[{label}]()")
        } else {
            caps[0].to_string()
        }
    });

    let step2 = HREF_DOUBLE_RE.replace_all(&step1, |caps: &Captures| {
        let raw_url = &caps[1];
        if is_disallowed_url(raw_url, &normalized_domains) {
            r#"href="""#.to_string()
        } else {
            caps[0].to_string()
        }
    });

    let step3 = HREF_SINGLE_RE.replace_all(&step2, |caps: &Captures| {
        let raw_url = &caps[1];
        if is_disallowed_url(raw_url, &normalized_domains) {
            "href=''".to_string()
        } else {
            caps[0].to_string()
        }
    });

    step3.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_exact_and_subdomain_matches_preserving_others() {
        let domains = vec!["internal.corp".to_string(), "Example.COM".to_string()];
        let input = concat!(
            "See [docs](https://wiki.internal.corp/page) and ",
            "[root](http://example.com/path) and ",
            "[public](https://rust-lang.org/learn) and ",
            "[rel](/local/path)."
        );

        let out = strip_disallowed_urls(input, &domains);
        assert_eq!(
            out,
            "See [docs]() and [root]() and [public](https://rust-lang.org/learn) and [rel](/local/path)."
        );
    }

    #[test]
    fn strips_html_href_attributes() {
        let domains = vec!["secret.local".to_string()];
        let input = r#"<a href="https://app.secret.local/id">A</a> <a href='https://secret.local'>B</a> <a href="/ok">C</a>"#;
        let out = strip_disallowed_urls(input, &domains);
        assert_eq!(
            out,
            r#"<a href="">A</a> <a href=''>B</a> <a href="/ok">C</a>"#
        );
    }
}

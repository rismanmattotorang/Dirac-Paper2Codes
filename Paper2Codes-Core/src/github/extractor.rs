//! GitHub URL extraction from paper text
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref GITHUB_URL_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"https?://github\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+").unwrap(),
        Regex::new(r"https?://github\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+[^\s]*").unwrap(),
        Regex::new(r"https?://github\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+/blob/[^\s]+").unwrap(),
        Regex::new(r"https?://github\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+/tree/[^\s]+").unwrap(),
        Regex::new(r"https?://raw\.githubusercontent\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+/[^\s]+")
            .unwrap(),
    ];
}

/// Extract GitHub URLs from text
pub fn extract_github_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();

    for pattern in GITHUB_URL_PATTERNS.iter() {
        for capture in pattern.find_iter(text) {
            let url = clean_github_url(capture.as_str());
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    }

    urls
}

/// Clean GitHub URL by removing trailing punctuation
fn clean_github_url(url: &str) -> String {
    url.trim()
        .trim_end_matches(|c: char| {
            matches!(
                c,
                '.' | ',' | ';' | ':' | ')' | ']' | '}' | '>' | '"' | '\''
            )
        })
        .to_string()
}

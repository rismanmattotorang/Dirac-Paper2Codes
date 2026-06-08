//! Validates real PDF text extraction against the bundled sample papers.
//!
//! These PDFs live at the repository root (one level above the crate). If they
//! are not present (e.g. a sparse checkout) the test skips rather than fails.

use paper2codes::document::DocumentParser;
use std::path::Path;

async fn extract(name: &str) -> Option<paper2codes::types::Paper> {
    let path = Path::new("..").join(name);
    if !path.exists() {
        eprintln!("skipping: {} not present", path.display());
        return None;
    }
    let parser = DocumentParser::new();
    Some(
        parser
            .parse_pdf(&path)
            .await
            .unwrap_or_else(|e| panic!("PDF extraction failed for {}: {}", path.display(), e)),
    )
}

#[tokio::test]
async fn extracts_substantial_text_from_bundled_pdf() {
    let Some(paper) = extract("Dirac-Paper2Codes.pdf").await else {
        return;
    };

    // Reconstruct the full text from segments to assert on real content.
    let full: String = paper
        .segments
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // A real paper yields a lot of text and recognisable domain vocabulary.
    assert!(
        full.len() > 2000,
        "expected substantial extracted text, got {} chars",
        full.len()
    );
    let lower = full.to_lowercase();
    assert!(
        ["paper", "code", "the", "and"]
            .iter()
            .any(|w| lower.contains(w)),
        "extracted text did not contain expected common words"
    );
}

#[tokio::test]
async fn extracts_from_second_bundled_pdf() {
    let Some(paper) = extract("Dirac-CoScientist.pdf").await else {
        return;
    };
    assert!(!paper.segments.is_empty(), "expected at least one segment");
}

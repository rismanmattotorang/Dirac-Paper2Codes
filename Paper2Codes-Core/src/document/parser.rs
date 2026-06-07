use crate::error::{DocumentError, Result};
use crate::types::{Paper, PaperMetadata, PaperSegment, SegmentType};
use std::path::Path;
use tracing::info;
use uuid::Uuid;

pub struct DocumentParser;

impl DocumentParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a PDF file and extract text content
    pub async fn parse_pdf(&self, path: &Path) -> Result<Paper> {
        info!("Parsing PDF: {:?}", path);

        // Try to extract text from PDF
        let text = Self::extract_pdf_text(path)?;

        // Parse the extracted text
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        self.parse_text(&text, title).await
    }

    /// Extract text from PDF file using the pdf crate
    /// Note: The pdf crate has limited text extraction capabilities.
    /// For production use, consider using pdf-extract or similar specialized library.
    fn extract_pdf_text(path: &Path) -> Result<String> {
        use pdf::file::FileOptions;
        use std::io::Read;

        info!("Extracting text from PDF: {:?}", path);

        // Read PDF file
        let mut file = std::fs::File::open(path)
            .map_err(|e| DocumentError::ParseFailed(format!("Failed to open PDF file: {}", e)))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| DocumentError::ParseFailed(format!("Failed to read PDF file: {}", e)))?;

        // Parse PDF to validate it's a valid PDF
        let _file = FileOptions::cached()
            .password(b"")
            .load(&buffer[..])
            .map_err(|e| DocumentError::ParseFailed(format!("Failed to parse PDF: {}", e)))?;

        // The pdf crate (v0.8) has limited text extraction capabilities.
        // For now, we return an error suggesting the user convert to text format.
        // In production, you should use a specialized PDF text extraction library like pdf-extract.
        Err(DocumentError::ParseFailed(
            format!(
                "PDF text extraction is not fully supported. Please convert the PDF to text format first.\n\
                You can use tools like pdftotext or online converters.\n\
                File: {:?}\n\
                Alternatively, use the parse_text() method with the extracted text content.",
                path
            )
        ).into())
    }

    /// Parse text content and create a Paper structure
    pub async fn parse_text(&self, content: &str, title: Option<String>) -> Result<Paper> {
        info!("Parsing text content, length: {} characters", content.len());

        if content.trim().is_empty() {
            return Err(DocumentError::ParseFailed("Empty content provided".to_string()).into());
        }

        // Extract title if not provided
        let title = title.unwrap_or_else(|| Self::extract_title(content));

        // Extract abstract with improved logic
        let abstract_text = Self::extract_abstract(content);

        // Extract metadata
        let metadata = Self::extract_metadata(content);

        // Create initial segments - will be refined by segmenter
        let segments = Self::create_initial_segments(content);

        info!("Created paper with {} initial segments", segments.len());

        Ok(Paper {
            id: Uuid::new_v4().to_string(),
            title,
            abstract_text,
            segments,
            algorithms: Vec::new(),
            equations: Vec::new(),
            figures: Vec::new(),
            tables: Vec::new(),
            references: Vec::new(),
            metadata,
        })
    }

    /// Extract title from content (first non-empty line or infer from content)
    fn extract_title(content: &str) -> String {
        // Try to find the first substantial line as title
        for line in content.lines().take(10) {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.len() > 10 && trimmed.len() < 200 {
                return trimmed.to_string();
            }
        }
        "Untitled Paper".to_string()
    }

    /// Extract abstract section from content with improved logic
    fn extract_abstract(content: &str) -> String {
        let content_lower = content.to_lowercase();

        // Look for "abstract" keyword with better pattern matching
        let abstract_patterns = ["abstract", "abstract:", "abstract\n", "abstract "];

        for pattern in &abstract_patterns {
            if let Some(abstract_start) = content_lower.find(pattern) {
                // Find the start of actual abstract content (skip the header)
                let actual_start = abstract_start + pattern.len();

                // Skip any whitespace or newlines after "abstract"
                let actual_start = content[actual_start..]
                    .char_indices()
                    .find(|(_, c)| !c.is_whitespace())
                    .map(|(idx, _)| actual_start + idx)
                    .unwrap_or(actual_start);

                // Find where the abstract ends (look for introduction or other sections)
                let end_markers = [
                    "introduction",
                    "1. introduction",
                    "1 introduction",
                    "\n\n1",
                    "\n\n2",
                    "keywords",
                    "background",
                    "related work",
                    "1. ",
                ];

                let mut abstract_end = content.len();
                for marker in end_markers.iter() {
                    if let Some(pos) = content_lower[actual_start..].find(marker) {
                        // Make sure we're at a section boundary (preceded by newline or start)
                        let marker_pos = actual_start + pos;
                        if marker_pos == actual_start
                            || content
                                .as_bytes()
                                .get(marker_pos.saturating_sub(1))
                                .map(|&b| b == b'\n' || b == b'\r')
                                .unwrap_or(true)
                        {
                            abstract_end = marker_pos;
                            break;
                        }
                    }
                }

                // Limit abstract to reasonable length (max 3000 chars)
                if abstract_end - actual_start > 3000 {
                    abstract_end = actual_start + 3000;
                }

                let abstract_text = content[actual_start..abstract_end].trim().to_string();
                if !abstract_text.is_empty() && abstract_text.len() > 20 {
                    return abstract_text;
                }
            }
        }

        // Fallback: use first few lines (skip title and empty lines)
        content
            .lines()
            .skip(1) // Skip title
            .filter(|l| !l.trim().is_empty())
            .take(15) // Take more lines for better coverage
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .chars()
            .take(800) // Increased from 500 for better coverage
            .collect()
    }

    /// Extract paper metadata (authors, year, etc.)
    fn extract_metadata(content: &str) -> PaperMetadata {
        let mut metadata = PaperMetadata {
            authors: Vec::new(),
            year: None,
            venue: None,
            keywords: Vec::new(),
            file_path: None,
        };

        // Try to extract year (look for 4-digit numbers that could be years)
        if let Some(year_match) = regex::Regex::new(r"\b(19|20)\d{2}\b")
            .ok()
            .and_then(|re| re.find(content))
        {
            if let Ok(year) = year_match.as_str().parse::<u32>() {
                metadata.year = Some(year);
            }
        }

        // Try to extract keywords
        let content_lower = content.to_lowercase();
        if let Some(keywords_start) = content_lower.find("keywords") {
            let keywords_section = &content[keywords_start..];
            if let Some(line_end) = keywords_section.find('\n') {
                let keywords_line = &keywords_section[9..line_end]; // Skip "keywords:"
                metadata.keywords = keywords_line
                    .split(|c| c == ',' || c == ';' || c == '.')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        metadata
    }

    /// Create initial segments from content
    fn create_initial_segments(content: &str) -> Vec<PaperSegment> {
        let line_count = content.lines().count();

        vec![PaperSegment {
            id: Uuid::new_v4().to_string(),
            section: "Full Text".to_string(),
            content: content.to_string(),
            segment_type: SegmentType::Other("Full".to_string()),
            embedding: None,
            line_range: (0, line_count),
        }]
    }
}

impl Default for DocumentParser {
    fn default() -> Self {
        Self::new()
    }
}

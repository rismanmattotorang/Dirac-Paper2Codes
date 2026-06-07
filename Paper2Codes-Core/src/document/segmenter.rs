use crate::error::Result;
use crate::types::{Paper, PaperSegment, SegmentType};
use regex::Regex;
use uuid::Uuid;

pub struct PaperSegmenter {
    section_pattern: Regex,
}

impl PaperSegmenter {
    pub fn new() -> Self {
        // Enhanced regex pattern for better section detection
        // Matches section headers with optional numbering, markdown headers, and common variations
        // Improved to handle more section name variations and edge cases
        let pattern = Regex::new(
            r"(?i)^\s*(?:#{1,6}\s*)?(?:\d+\.?\s*)?(abstract|introduction|intro|related\s+work|related\s+works|background|methodology|method|methods|algorithm|algorithms|experiment|experiments|experimental|evaluation|results|result|discussion|conclusion|conclusions|references?|bibliography|appendix|appendices|acknowledgments?|acknowledgement)\s*:?\s*$"
        ).unwrap_or_else(|_| {
            // Fallback to empty regex that matches nothing
            Regex::new("^$").expect("Empty regex should always compile")
        });
        Self {
            section_pattern: pattern,
        }
    }

    pub fn segment(&self, paper: &mut Paper) -> Result<()> {
        let mut segments = Vec::new();
        let mut current_section = "Introduction".to_string();
        let mut current_content = String::new();
        let mut line_num = 0;
        let mut abstract_added = false;

        // Process abstract separately if it exists
        if !paper.abstract_text.trim().is_empty() {
            let abstract_content = paper.abstract_text.trim().to_string();
            let abstract_line_count = abstract_content.lines().count();
            segments.push(PaperSegment {
                id: Uuid::new_v4().to_string(),
                section: "Abstract".to_string(),
                content: abstract_content,
                segment_type: SegmentType::Abstract,
                embedding: None,
                line_range: (0, abstract_line_count),
            });
            line_num = abstract_line_count;
            abstract_added = true;
        }

        // Process main content from segments
        let full_text = paper
            .segments
            .iter()
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // If we have full text, process it for sections
        if !full_text.trim().is_empty() {
            current_content.clear();
            let mut line_start = line_num;
            let mut in_abstract = false;

            for line in full_text.lines() {
                let trimmed_line = line.trim();

                // Skip empty lines at section boundaries
                if trimmed_line.is_empty() && current_content.trim().is_empty() {
                    line_num += 1;
                    continue;
                }

                // Check if this is a section header
                if let Some(captures) = self.section_pattern.captures(line) {
                    // Save previous section if it has content
                    if !current_content.trim().is_empty() && !in_abstract {
                        let segment_type = self.classify_section(&current_section);
                        segments.push(PaperSegment {
                            id: Uuid::new_v4().to_string(),
                            section: current_section.clone(),
                            content: current_content.trim().to_string(),
                            segment_type,
                            embedding: None,
                            line_range: (line_start, line_num),
                        });
                    }

                    // Start new section
                    if let Some(section_match) = captures.get(1) {
                        let section_name = section_match.as_str().trim().to_string();
                        current_section = section_name.clone();
                        current_content.clear();
                        line_start = line_num;
                        in_abstract = section_name.to_lowercase() == "abstract" && !abstract_added;
                    }
                } else {
                    // Add line to current section content
                    if !in_abstract || !abstract_added {
                        current_content.push_str(line);
                        current_content.push('\n');
                    }
                }
                line_num += 1;
            }

            // Add final section if it has content
            if !current_content.trim().is_empty() && !in_abstract {
                let segment_type = self.classify_section(&current_section);
                segments.push(PaperSegment {
                    id: Uuid::new_v4().to_string(),
                    section: current_section,
                    content: current_content.trim().to_string(),
                    segment_type,
                    embedding: None,
                    line_range: (line_start, line_num),
                });
            }
        }

        // If no segments were created, create a single segment from the full text
        if segments.is_empty() {
            let content = if !paper.abstract_text.is_empty() {
                format!("{}\n\n{}", paper.abstract_text, full_text)
            } else {
                full_text
            };

            if !content.trim().is_empty() {
                segments.push(PaperSegment {
                    id: Uuid::new_v4().to_string(),
                    section: "Full Text".to_string(),
                    content: content.trim().to_string(),
                    segment_type: SegmentType::Other("Full".to_string()),
                    embedding: None,
                    line_range: (0, content.lines().count()),
                });
            }
        }

        paper.segments = segments;
        Ok(())
    }

    fn classify_section(&self, section: &str) -> SegmentType {
        let section_lower = section.to_lowercase();
        match section_lower.as_str() {
            "abstract" => SegmentType::Abstract,
            "introduction" | "intro" => SegmentType::Introduction,
            "methodology" | "method" | "methods" => SegmentType::Methodology,
            "algorithm" | "algorithms" => SegmentType::Algorithm,
            "experiment" | "experiments" | "evaluation" => SegmentType::Experiment,
            "results" | "result" => SegmentType::Results,
            "conclusion" | "conclusions" => SegmentType::Conclusion,
            "related work" | "related works" | "background" => {
                SegmentType::Other("Related Work".to_string())
            }
            "discussion" => SegmentType::Other("Discussion".to_string()),
            "references" | "reference" | "bibliography" => {
                SegmentType::Other("References".to_string())
            }
            "appendix" | "appendices" => SegmentType::Other("Appendix".to_string()),
            _ => SegmentType::Other(section.to_string()),
        }
    }
}

impl Default for PaperSegmenter {
    fn default() -> Self {
        Self::new()
    }
}

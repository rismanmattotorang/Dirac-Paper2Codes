use crate::types::{Algorithm, Equation, Paper};
use regex::Regex;
use uuid::Uuid;

pub struct ContentExtractor {
    equation_pattern: Regex,
}

impl ContentExtractor {
    pub fn new() -> Self {
        // Enhanced equation pattern: matches LaTeX equations, inline math, and mathematical expressions
        // Includes both display and inline math, numbered equations, and common LaTeX environments
        // Improved to handle nested delimiters and more LaTeX constructs
        let equation_pattern = Regex::new(
            r"(?s)(?:\$\$[^$]+\$\$)|(?:\$[^$]+\$)|(?:\\begin\{(?:equation|align|eqnarray|multline|gather|split|cases|matrix|pmatrix|bmatrix|vmatrix)\*?\}.*?\\end\{(?:equation|align|eqnarray|multline|gather|split|cases|matrix|pmatrix|bmatrix|vmatrix)\*?\})|(?:\\\[.*?\\\])|(?:\\\(.*?\\\))|(?:equation\s*(?:\d+|[A-Z]\d*)[:\s]*[^\n]+)"
        ).unwrap_or_else(|_| {
            Regex::new("^$").expect("Empty regex should always compile")
        });

        Self { equation_pattern }
    }

    pub fn extract_algorithms(&self, paper: &Paper) -> Vec<Algorithm> {
        let mut algorithms = Vec::new();
        let mut processed_ranges = Vec::new();

        for segment in &paper.segments {
            let lines: Vec<&str> = segment.content.lines().collect();
            let mut idx = 0;

            while idx < lines.len() {
                let line = lines[idx];
                let line_lower = line.to_lowercase();

                // Check if this is an algorithm header (more flexible matching)
                // Improved pattern matching for better detection
                let is_algorithm_header = (line_lower.contains("algorithm")
                    || line_lower.contains("procedure")
                    || line_lower.contains("pseudocode")
                    || line_lower.contains("alg.")
                    || (line_lower.contains("function")
                        && (line_lower.matches(char::is_numeric).count() > 0
                            || line_lower.contains("function"))))
                    && !Self::already_processed(idx, &processed_ranges);

                if is_algorithm_header {
                    // Extract algorithm name
                    let algorithm_name = self
                        .extract_algorithm_name(line)
                        .unwrap_or_else(|| format!("Algorithm {}", algorithms.len() + 1));

                    // Extract the algorithm block
                    let mut algorithm_lines = Vec::new();
                    let start_idx = idx;

                    // Include the header line
                    algorithm_lines.push(line);
                    idx += 1;

                    // Continue extracting lines until we hit an end condition
                    let mut consecutive_empty = 0;
                    while idx < lines.len() {
                        let curr_line = lines[idx];

                        // Stop conditions
                        if Self::is_algorithm_end_marker(curr_line) {
                            algorithm_lines.push(curr_line);
                            idx += 1;
                            break;
                        }

                        // Track consecutive empty lines
                        if curr_line.trim().is_empty() {
                            consecutive_empty += 1;
                            if consecutive_empty >= 2 && algorithm_lines.len() > 3 {
                                break; // End of algorithm block
                            }
                        } else {
                            consecutive_empty = 0;
                        }

                        // Check if next line starts a new section
                        let next_line_lower = curr_line.to_lowercase();
                        if Self::is_section_header(&next_line_lower) && algorithm_lines.len() > 3 {
                            break;
                        }

                        algorithm_lines.push(curr_line);
                        idx += 1;

                        // Safety: don't extract more than 100 lines
                        if algorithm_lines.len() > 100 {
                            break;
                        }
                    }

                    // Only add if we got substantial content
                    if algorithm_lines.len() >= 3 {
                        let pseudocode = algorithm_lines.join("\n").trim().to_string();
                        let description = Self::extract_algorithm_description(&algorithm_lines);

                        algorithms.push(Algorithm {
                            id: Uuid::new_v4().to_string(),
                            name: algorithm_name,
                            pseudocode,
                            description,
                            line_range: (
                                start_idx + segment.line_range.0,
                                idx + segment.line_range.0,
                            ),
                        });

                        processed_ranges.push((start_idx, idx));
                    }
                } else {
                    idx += 1;
                }
            }
        }

        algorithms
    }

    fn already_processed(idx: usize, ranges: &[(usize, usize)]) -> bool {
        ranges
            .iter()
            .any(|(start, end)| idx >= *start && idx < *end)
    }

    fn is_algorithm_end_marker(line: &str) -> bool {
        let line_lower = line.to_lowercase().trim().to_string();
        line_lower == "end"
            || line_lower == "end algorithm"
            || line_lower == "end procedure"
            || line_lower == "end function"
            || line_lower == "end pseudocode"
            || line_lower.starts_with("end ")
            || line_lower == "return"
            || (line_lower.starts_with("algorithm") && line_lower.contains("end"))
    }

    fn is_section_header(line: &str) -> bool {
        let sections = [
            "abstract",
            "introduction",
            "intro",
            "related work",
            "background",
            "methodology",
            "method",
            "methods",
            "experiment",
            "experiments",
            "evaluation",
            "results",
            "result",
            "discussion",
            "conclusion",
            "conclusions",
            "references",
            "bibliography",
            "appendix",
        ];

        let trimmed = line.trim().to_lowercase();
        sections.iter().any(|s| {
            trimmed == *s
                || trimmed.starts_with(&format!("{}:", s))
                || trimmed.starts_with(&format!("{} ", s))
                || (trimmed.len() > s.len()
                    && trimmed.starts_with(s)
                    && trimmed
                        .chars()
                        .nth(s.len())
                        .map_or(false, |c| c == ':' || c == ' ' || c.is_whitespace()))
        })
    }

    fn extract_algorithm_description(lines: &[&str]) -> String {
        // Look for lines after the header that might be description
        if lines.len() > 1 {
            let mut description_lines = Vec::new();
            for line in lines.iter().skip(1).take(3) {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && !trimmed.starts_with("Input:")
                    && !trimmed.starts_with("Output:")
                    && !trimmed.starts_with("1")
                    && !trimmed.starts_with("for ")
                    && !trimmed.starts_with("while ")
                {
                    description_lines.push(trimmed);
                } else {
                    break;
                }
            }
            if !description_lines.is_empty() {
                return description_lines.join(" ");
            }
        }
        String::new()
    }

    pub fn extract_equations(&self, paper: &Paper) -> Vec<Equation> {
        let mut equations = Vec::new();

        for segment in &paper.segments {
            let content = &segment.content;
            let line_num = segment.line_range.0;

            // Method 1: Extract LaTeX equations
            for cap in self.equation_pattern.find_iter(content) {
                let latex = cap.as_str().to_string();

                // Try to find the line number
                let before_match = &content[..cap.start()];
                let match_line_num = line_num + before_match.matches('\n').count();

                // Try to extract description from surrounding text
                let description = Self::extract_equation_description(content, cap.start());

                equations.push(Equation {
                    id: Uuid::new_v4().to_string(),
                    latex,
                    description,
                    line_range: (match_line_num, match_line_num),
                });
            }

            // Method 2: Extract numbered equations
            let lines: Vec<&str> = content.lines().collect();
            for (idx, line) in lines.iter().enumerate() {
                if Self::looks_like_equation(line) {
                    let latex = line.trim().to_string();

                    // Get description from previous or next lines
                    let mut description = String::new();
                    if idx > 0 {
                        let prev_line = lines[idx - 1].trim();
                        if !prev_line.is_empty() && prev_line.len() < 200 {
                            description = prev_line.to_string();
                        }
                    }

                    // Avoid duplicates
                    if !equations.iter().any(|e| e.latex.contains(&latex)) {
                        equations.push(Equation {
                            id: Uuid::new_v4().to_string(),
                            latex,
                            description,
                            line_range: (line_num + idx, line_num + idx),
                        });
                    }
                }
            }
        }

        // Deduplicate equations
        Self::deduplicate_equations(&mut equations);

        equations
    }

    fn extract_equation_description(content: &str, equation_pos: usize) -> String {
        // Look for text before the equation that might be a description
        let before = &content[..equation_pos];
        let lines: Vec<&str> = before.lines().collect();

        if let Some(last_line) = lines.last() {
            let trimmed = last_line.trim();
            if !trimmed.is_empty() && trimmed.len() < 200 {
                return trimmed.to_string();
            }
        }

        String::new()
    }

    fn looks_like_equation(line: &str) -> bool {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            return false;
        }

        // Check for mathematical symbols and patterns
        let has_math_symbols = trimmed.contains('=')
            || trimmed.contains('+')
            || trimmed.contains('-')
            || trimmed.contains('×')
            || trimmed.contains('·')
            || trimmed.contains('∑')
            || trimmed.contains('∫')
            || trimmed.contains('α')
            || trimmed.contains('β')
            || trimmed.contains('θ')
            || trimmed.contains('Σ')
            || trimmed.contains('π')
            || trimmed.contains('∞')
            || trimmed.contains('√')
            || trimmed.contains('∂')
            || trimmed.contains('∇')
            || trimmed.contains('∈')
            || trimmed.contains('≤')
            || trimmed.contains('≥')
            || trimmed.contains('≠')
            || trimmed.contains('≈');

        // Check for LaTeX-like patterns
        let has_latex = trimmed.contains('\\')
            || trimmed.starts_with('$')
            || trimmed.contains("\\frac")
            || trimmed.contains("\\sum")
            || trimmed.contains("\\int")
            || trimmed.contains("\\sqrt");

        // Check for equation numbering patterns
        let has_equation_number = Regex::new(r"\((\d+)\)\s*$|\[(\d+)\]\s*$")
            .map(|re| re.is_match(trimmed))
            .unwrap_or(false);

        // Should be substantial but not too long
        let reasonable_length = trimmed.len() > 3 && trimmed.len() < 500;

        // Should not be regular prose (fewer words, more symbols)
        let word_count = trimmed.split_whitespace().count();
        let not_prose = word_count < 15
            || trimmed.chars().filter(|c| c.is_alphabetic()).count()
                < trimmed
                    .chars()
                    .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
                    .count();

        // Check for common equation patterns
        let has_equation_pattern = trimmed.contains(" = ")
            || trimmed.contains(" := ")
            || trimmed.contains(" := ")
            || Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*\s*[=:]\s*")
                .map(|re| re.is_match(trimmed))
                .unwrap_or(false);

        (has_math_symbols || has_latex || has_equation_number || has_equation_pattern)
            && reasonable_length
            && not_prose
    }

    fn deduplicate_equations(equations: &mut Vec<Equation>) {
        let mut seen = std::collections::HashSet::new();
        equations.retain(|eq| {
            let normalized = eq.latex.trim().to_lowercase();
            if seen.contains(&normalized) {
                false
            } else {
                seen.insert(normalized);
                true
            }
        });
    }

    fn extract_algorithm_name(&self, line: &str) -> Option<String> {
        // Try to extract algorithm name from line like "Algorithm 1: Name"
        if let Some(captures) = Regex::new(r"(?i)algorithm\s+\d+[:\s]+(.+)")
            .ok()?
            .captures(line)
        {
            return Some(captures.get(1)?.as_str().trim().to_string());
        }
        None
    }
}

impl Default for ContentExtractor {
    fn default() -> Self {
        Self::new()
    }
}

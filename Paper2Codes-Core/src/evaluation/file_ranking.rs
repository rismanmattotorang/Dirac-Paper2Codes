//! File ranking by semantic relevance
use crate::error::Result;
use std::path::PathBuf;

/// Rank files by keyword relevance to a query with improved scoring
pub async fn rank_files_by_relevance(
    files: &[PathBuf],
    query: &str,
    max_files: usize,
) -> Result<Vec<PathBuf>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let query_lower = query.to_lowercase();
    // Extract meaningful words (filter out common stop words)
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "as", "is", "was", "are", "were", "be", "been", "have", "has", "had", "do", "does",
        "did", "will", "would", "should", "could", "may", "might", "must", "can", "this", "that",
        "these", "those", "i", "you", "he", "she", "it", "we", "they",
    ]
    .iter()
    .cloned()
    .collect();

    let query_words: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|w| w.len() > 2 && !stop_words.contains(*w))
        .collect();

    if query_words.is_empty() {
        // If no meaningful words, return files as-is
        return Ok(files.iter().take(max_files).cloned().collect());
    }

    // Score each file
    let mut scored_files: Vec<(PathBuf, f64)> = Vec::new();

    for file_path in files {
        let mut score = 0.0;

        // Score based on filename (higher weight for exact matches)
        if let Some(file_name) = file_path.file_name() {
            let name_lower = file_name.to_string_lossy().to_lowercase();
            for word in &query_words {
                if name_lower == *word {
                    score += 5.0; // Exact filename match
                } else if name_lower.contains(word) {
                    score += 2.0; // Partial match
                }
            }
        }

        // Score based on file content (read with error handling)
        // Use async file reading for better performance
        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                // Use a more efficient approach: single pass through content
                let content_lower = content.to_lowercase();
                // Sample first 10000 chars for better coverage while maintaining performance
                let sample_len = content_lower.len().min(10000);
                let content_sample = &content_lower[..sample_len];

                // Count matches for all query words in a single pass
                let mut word_matches = std::collections::HashMap::new();
                for word in &query_words {
                    word_matches.insert(*word, 0);
                }

                // Single pass through content to count all word matches
                for word in &query_words {
                    let matches = content_sample.matches(word).count();
                    if matches > 0 {
                        word_matches.insert(*word, matches);
                        // Logarithmic scaling to prevent one word from dominating
                        score += (matches as f64).ln_1p() * 0.5;
                    }
                }

                // Bonus for files that contain all query words
                let words_found = word_matches.values().filter(|&&count| count > 0).count();
                if words_found == query_words.len() && !query_words.is_empty() {
                    score += 3.0;
                }

                // Additional bonus for high match density
                let total_matches: usize = word_matches.values().sum();
                if total_matches > 10 {
                    score += 1.0;
                }
            }
            Err(_) => {
                // File read error - give it a small score but don't exclude it
                // This could be a binary file or permission issue
                score += 0.1;
            }
        }

        scored_files.push((file_path.clone(), score));
    }

    // Sort by score (descending) and return top-k
    scored_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(scored_files
        .into_iter()
        .take(max_files)
        .map(|(path, _)| path)
        .collect())
}

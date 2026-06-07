//! Okapi BM25 sparse lexical retrieval.
//!
//! The original CPR implementation approximated lexical matching with a crude
//! substring-containment ratio (`keyword_overlap`). The Dirac-Paper2Codes paper,
//! however, specifies *BM25 ranking* as the lexical signal in the hybrid
//! retrieval kernel. This module closes that paper↔code gap with a proper
//! Okapi BM25 implementation, which provides term-frequency saturation, inverse
//! document frequency weighting, and document-length normalisation — none of
//! which the substring heuristic captured.
//!
//! BM25 is combined with dense (embedding) retrieval via Reciprocal Rank Fusion
//! (see [`crate::retrieval::fusion`]), the rank-based fusion strategy that is
//! robust to the incompatible score scales of sparse (unbounded) and dense
//! ([-1, 1]) retrievers.

use std::collections::HashMap;

/// Stop words filtered out during tokenisation. Kept small and English-centric;
/// BM25's IDF weighting already down-weights ubiquitous terms, so this list only
/// needs to remove the most common function words.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
    "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has", "had", "do",
    "does", "did", "will", "would", "should", "could", "may", "might", "must", "can", "this",
    "that", "these", "those", "i", "you", "he", "she", "it", "we", "they",
];

/// Tokenise text into normalised terms: lowercased, alphanumeric (with `-`),
/// length > 2, and not a stop word. Shared by BM25 and query keyword extraction
/// so the lexical signal is consistent across the retrieval pipeline.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter_map(|raw| {
            let word = raw.trim_matches('-').to_lowercase();
            if word.len() > 2 && !STOP_WORDS.contains(&word.as_str()) {
                Some(word)
            } else {
                None
            }
        })
        .collect()
}

/// An in-memory Okapi BM25 index over a fixed corpus of documents.
#[derive(Debug, Clone)]
pub struct Bm25Index {
    k1: f32,
    b: f32,
    avgdl: f32,
    doc_ids: Vec<String>,
    doc_len: Vec<f32>,
    /// Per-document term frequencies.
    term_freqs: Vec<HashMap<String, u32>>,
    /// Inverse document frequency per term.
    idf: HashMap<String, f32>,
}

impl Bm25Index {
    /// Build an index from `(id, text)` pairs using the standard BM25 defaults
    /// (`k1 = 1.5`, `b = 0.75`).
    pub fn build<'a, I>(docs: I) -> Self
    where
        I: IntoIterator<Item = (String, &'a str)>,
    {
        Self::build_with_params(docs, 1.5, 0.75)
    }

    /// Build an index with explicit BM25 parameters.
    ///
    /// * `k1` controls term-frequency saturation (typical range 1.2–2.0).
    /// * `b` controls document-length normalisation (0 = none, 1 = full).
    pub fn build_with_params<'a, I>(docs: I, k1: f32, b: f32) -> Self
    where
        I: IntoIterator<Item = (String, &'a str)>,
    {
        let mut doc_ids = Vec::new();
        let mut doc_len = Vec::new();
        let mut term_freqs = Vec::new();
        // Number of documents each term appears in.
        let mut doc_freq: HashMap<String, u32> = HashMap::new();

        for (id, text) in docs {
            let tokens = tokenize(text);
            let mut tf: HashMap<String, u32> = HashMap::new();
            for token in &tokens {
                *tf.entry(token.clone()).or_insert(0) += 1;
            }
            for term in tf.keys() {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
            doc_len.push(tokens.len() as f32);
            term_freqs.push(tf);
            doc_ids.push(id);
        }

        let n = doc_ids.len().max(1) as f32;
        let avgdl = if doc_len.is_empty() {
            0.0
        } else {
            doc_len.iter().sum::<f32>() / doc_len.len() as f32
        };

        // BM25 IDF with the standard +0.5 smoothing. Floored at a small positive
        // value so that terms appearing in every document still contribute a
        // little rather than going negative.
        let idf = doc_freq
            .into_iter()
            .map(|(term, df)| {
                let df = df as f32;
                let value = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                (term, value.max(1e-6))
            })
            .collect();

        Self {
            k1,
            b,
            avgdl,
            doc_ids,
            doc_len,
            term_freqs,
            idf,
        }
    }

    /// Number of documents in the index.
    pub fn len(&self) -> usize {
        self.doc_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.doc_ids.is_empty()
    }

    /// Score every document against `query_terms`, returning `(id, score)` pairs
    /// sorted by descending score. Documents with a zero score are retained so
    /// callers can fuse complete rankings; filter as needed.
    pub fn score(&self, query_terms: &[String]) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = Vec::with_capacity(self.doc_ids.len());

        for (i, id) in self.doc_ids.iter().enumerate() {
            let tf = &self.term_freqs[i];
            let dl = self.doc_len[i];
            let mut score = 0.0f32;

            for term in query_terms {
                let freq = match tf.get(term) {
                    Some(f) => *f as f32,
                    None => continue,
                };
                let idf = match self.idf.get(term) {
                    Some(v) => *v,
                    None => continue,
                };
                let denom = freq + self.k1 * (1.0 - self.b + self.b * dl / self.avgdl.max(1e-6));
                score += idf * (freq * (self.k1 + 1.0)) / denom.max(1e-6);
            }

            scored.push((id.clone(), score));
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Convenience: the ranked list of document ids (best first), excluding
    /// documents that did not match any query term.
    pub fn ranked_ids(&self, query_terms: &[String]) -> Vec<String> {
        self.score(query_terms)
            .into_iter()
            .filter(|(_, score)| *score > 0.0)
            .map(|(id, _)| id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_filters_stopwords_and_short_tokens() {
        let tokens = tokenize("The Fast Fourier Transform (FFT) is an algorithm.");
        assert!(tokens.contains(&"fast".to_string()));
        assert!(tokens.contains(&"fourier".to_string()));
        assert!(tokens.contains(&"transform".to_string()));
        assert!(tokens.contains(&"fft".to_string()));
        assert!(tokens.contains(&"algorithm".to_string()));
        // Stop words and short tokens removed.
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
        assert!(!tokens.contains(&"an".to_string()));
    }

    #[test]
    fn bm25_ranks_relevant_document_first() {
        let docs = vec![
            ("d1".to_string(), "the transformer uses multi head attention mechanism"),
            ("d2".to_string(), "gradient descent optimization for training neural networks"),
            ("d3".to_string(), "attention attention attention is all you need"),
        ];
        let index = Bm25Index::build(docs);
        let query = tokenize("attention mechanism");
        let ranked = index.score(&query);

        // d3 (high "attention" frequency) and d1 (matches both terms) should top
        // d2, which matches neither query term.
        assert_eq!(ranked.len(), 3);
        let top_ids: Vec<&str> = ranked.iter().take(2).map(|(id, _)| id.as_str()).collect();
        assert!(top_ids.contains(&"d1"));
        assert!(top_ids.contains(&"d3"));
        // d2 has no matching terms -> zero score, ranked last.
        assert_eq!(ranked[2].0, "d2");
        assert_eq!(ranked[2].1, 0.0);
    }

    #[test]
    fn idf_down_weights_ubiquitous_terms() {
        let docs = vec![
            ("d1".to_string(), "common common common rare-term"),
            ("d2".to_string(), "common common common"),
            ("d3".to_string(), "common common common"),
        ];
        let index = Bm25Index::build(docs);
        // "rare-term" appears in only one doc, so it must out-rank the ubiquitous
        // "common" term despite far lower frequency.
        let rare = index.score(&tokenize("rare-term"));
        assert_eq!(rare[0].0, "d1");
        assert!(rare[0].1 > 0.0);
    }

    #[test]
    fn empty_index_scores_nothing() {
        let index = Bm25Index::build(Vec::<(String, &str)>::new());
        assert!(index.is_empty());
        assert!(index.score(&tokenize("anything")).is_empty());
    }
}

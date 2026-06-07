//! Advanced search handlers for Phase 5
//!
//! Implements semantic search, filtering, sorting, faceted search, and suggestions

use axum::{
    extract::{Extension, Query},
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn};

use crate::api::state::AppState;
use crate::api::types::responses::ApiResponse;
use crate::error::{Paper2CodesError, Result};
use crate::storage::filters::PaperFilters;
use crate::types::{Paper, PaperSegment, SegmentType};

const MAX_SUGGESTIONS: usize = 5;

/// Advanced search request
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub filters: Option<SearchFilters>,
    pub sort: Option<SearchSort>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

/// Search filters
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    pub paper_ids: Option<Vec<String>>,
    pub domains: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub status: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

/// Search sort options
#[derive(Debug, Serialize, Deserialize)]
pub enum SearchSort {
    #[serde(rename = "relevance")]
    Relevance,
    #[serde(rename = "date_desc")]
    DateDescending,
    #[serde(rename = "date_asc")]
    DateAscending,
    #[serde(rename = "title")]
    Title,
}

/// Search result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResultItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f32,
    pub paper_id: Option<String>,
    pub paper_title: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Search response with facets
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultItem>,
    pub facets: SearchFacets,
    pub total: usize,
}

/// Search facets for filtering
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SearchFacets {
    pub domains: Vec<FacetItem>,
    pub authors: Vec<FacetItem>,
    pub years: Vec<FacetItem>,
    pub statuses: Vec<FacetItem>,
}

/// Facet item with count
#[derive(Debug, Serialize, Deserialize)]
pub struct FacetItem {
    pub value: String,
    pub count: usize,
}

/// Search suggestion
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub count: usize,
}

/// Advanced search endpoint
///
/// Endpoint: POST /api/search
pub async fn advanced_search(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<ApiResponse<SearchResponse>>> {
    info!(
        "Advanced search request: query={}, filters={:?}",
        request.query, request.filters
    );
    let query = request.query.trim();

    if query.is_empty() {
        return Err(Paper2CodesError::Validation(
            "Search query cannot be empty".to_string(),
        ));
    }

    let tokens = tokenize(query);
    if tokens.is_empty() {
        return Err(Paper2CodesError::Validation(
            "Search query must contain alphanumeric characters".to_string(),
        ));
    }

    let filters = request.filters.unwrap_or_default();
    let sort = request.sort.unwrap_or(SearchSort::Relevance);
    let page = request.page.unwrap_or(1).max(1);
    let per_page = request.per_page.unwrap_or(20).clamp(1, 100);

    let dataset = load_search_dataset(&state).await?;
    if dataset.is_empty() {
        warn!("Advanced search requested but no papers are indexed (storage disabled or empty)");
        let empty = SearchResponse {
            results: Vec::new(),
            facets: SearchFacets::default(),
            total: 0,
        };
        return Ok(Json(ApiResponse::new(empty)));
    }

    let (mut ranked, facets) = rank_results(&dataset, &tokens, &filters, &sort);
    let total = ranked.len();

    let start = (page - 1) * per_page;
    let end = (start + per_page).min(total);
    if start >= total {
        ranked.clear();
    } else {
        ranked = ranked[start..end].to_vec();
    }

    let results = ranked.into_iter().map(|candidate| candidate.item).collect();

    let response = SearchResponse {
        results,
        facets,
        total,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// Get search suggestions
///
/// Endpoint: GET /api/search/suggestions?q=...
pub async fn get_search_suggestions(
    Query(params): Query<HashMap<String, String>>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<SearchSuggestion>>>> {
    let query = params.get("q").cloned().unwrap_or_default();
    info!("Search suggestions request: q={}", query);

    let dataset = load_search_dataset(&state).await?;
    if dataset.is_empty() {
        warn!("Search suggestions requested but no papers available");
        return Ok(Json(ApiResponse::new(Vec::new())));
    }

    let suggestions = build_suggestions(&dataset, &query, MAX_SUGGESTIONS);

    Ok(Json(ApiResponse::new(suggestions)))
}

/// Get search facets
///
/// Endpoint: GET /api/search/facets?query=...
pub async fn get_search_facets(
    Query(params): Query<HashMap<String, String>>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<ApiResponse<SearchFacets>>> {
    let query = params.get("query").cloned().unwrap_or_default();
    info!("Search facets request: query={}", query);

    let dataset = load_search_dataset(&state).await?;
    if dataset.is_empty() {
        warn!("Search facets requested but dataset is empty");
        return Ok(Json(ApiResponse::new(SearchFacets::default())));
    }

    let tokens = tokenize(&query);
    let (candidates, candidate_facets) = rank_results(
        &dataset,
        &tokens,
        &SearchFilters::default(),
        &SearchSort::Relevance,
    );

    // When query is empty we want facets for the entire dataset.
    // Otherwise use the facets derived from the ranked candidates.
    let facets = if tokens.is_empty() {
        build_facets_for_dataset(&dataset)
    } else if candidates.is_empty() {
        SearchFacets::default()
    } else {
        candidate_facets
    };

    Ok(Json(ApiResponse::new(facets)))
}

/// Ranked search candidate with additional metadata for sorting and facets
#[derive(Clone)]
struct RankedCandidate {
    item: SearchResultItem,
    score: f32,
    year: Option<u32>,
    title: String,
    status: String,
    domains: Vec<String>,
    authors: Vec<String>,
}

async fn load_search_dataset(state: &Arc<AppState>) -> Result<Vec<Paper>> {
    let storage_guard = state.storage.read().await;
    if let Some(storage) = storage_guard.as_ref() {
        match storage.list_papers(PaperFilters::default()).await {
            Ok(papers) => Ok(papers),
            Err(err) => {
                warn!("Unable to load papers for search: {}", err);
                Ok(Vec::new())
            }
        }
    } else {
        warn!("Storage unavailable for search (falling back to empty dataset)");
        Ok(Vec::new())
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(|token| token.to_lowercase())
        .collect()
}

fn rank_results(
    dataset: &[Paper],
    tokens: &[String],
    filters: &SearchFilters,
    sort: &SearchSort,
) -> (Vec<RankedCandidate>, SearchFacets) {
    let mut candidates = Vec::new();

    for paper in dataset {
        if !paper_matches_filters(paper, filters) {
            continue;
        }

        for segment in &paper.segments {
            let score = compute_relevance(tokens, paper, segment);
            if score <= 0.0 {
                continue;
            }

            let snippet = extract_snippet(segment, tokens);
            let title = format!("{} — {}", paper.title, segment.section);

            let item = SearchResultItem {
                id: segment.id.clone(),
                title: title.clone(),
                content: snippet,
                score,
                paper_id: Some(paper.id.clone()),
                paper_title: Some(paper.title.clone()),
                metadata: build_metadata_map(paper, segment, score),
            };

            candidates.push(RankedCandidate {
                item,
                score,
                year: paper.metadata.year,
                title,
                status: derive_status(paper),
                domains: derive_domains(paper),
                authors: paper.metadata.authors.clone(),
            });
        }
    }

    sort_candidates(&mut candidates, sort);

    let facets = build_facets_from_candidates(&candidates);

    (candidates, facets)
}

fn compute_relevance(tokens: &[String], paper: &Paper, segment: &PaperSegment) -> f32 {
    if tokens.is_empty() {
        // Allow browsing without query but rank by metadata weight
        return 0.1;
    }

    let segment_text = segment.content.to_lowercase();
    let mut score = 0.0;

    for token in tokens {
        if segment_text.contains(token) {
            let frequency = segment_text.matches(token).count() as f32;
            score += 1.0 + (frequency - 1.0).max(0.0) * 0.35;
        }
    }

    if paper.title.to_lowercase().contains(&tokens.join(" ")) {
        score += 0.75;
    }

    for keyword in &paper.metadata.keywords {
        let keyword_lower = keyword.to_lowercase();
        if tokens.iter().any(|token| keyword_lower.contains(token)) {
            score += 0.4;
        }
    }

    match segment.segment_type {
        SegmentType::Algorithm | SegmentType::Methodology => {
            score *= 1.2;
        }
        SegmentType::Results | SegmentType::Experiment => {
            score *= 1.1;
        }
        _ => {}
    }

    score
}

fn extract_snippet(segment: &PaperSegment, tokens: &[String]) -> String {
    if tokens.is_empty() {
        return segment.content.chars().take(400).collect();
    }

    let lower_content = segment.content.to_lowercase();
    for token in tokens {
        if let Some(pos) = lower_content.find(token) {
            let start = pos.saturating_sub(120);
            let end = (pos + token.len() + 120).min(segment.content.len());
            let snippet = &segment.content[start..end];
            return snippet.trim().to_string();
        }
    }

    segment.content.chars().take(400).collect()
}

fn build_metadata_map(
    paper: &Paper,
    segment: &PaperSegment,
    score: f32,
) -> HashMap<String, serde_json::Value> {
    let mut metadata = HashMap::new();
    metadata.insert("paper_id".to_string(), json!(paper.id));
    metadata.insert("paper_title".to_string(), json!(paper.title));
    metadata.insert("section".to_string(), json!(segment.section));
    metadata.insert(
        "segment_type".to_string(),
        json!(format!("{:?}", segment.segment_type)),
    );
    metadata.insert("score".to_string(), json!(score));
    metadata.insert("authors".to_string(), json!(paper.metadata.authors));
    metadata.insert("keywords".to_string(), json!(paper.metadata.keywords));
    if let Some(year) = paper.metadata.year {
        metadata.insert("year".to_string(), json!(year));
    }

    metadata
}

fn paper_matches_filters(paper: &Paper, filters: &SearchFilters) -> bool {
    if let Some(ids) = &filters.paper_ids {
        if !ids.iter().any(|id| id == &paper.id) {
            return false;
        }
    }

    if let Some(domains) = &filters.domains {
        if !domains.is_empty() {
            let domains_lower: HashSet<String> = domains.iter().map(|d| d.to_lowercase()).collect();
            let paper_domains: HashSet<String> = paper
                .metadata
                .keywords
                .iter()
                .map(|k| k.to_lowercase())
                .collect();
            if paper_domains.is_empty() || domains_lower.is_disjoint(&paper_domains) {
                return false;
            }
        }
    }

    if let Some(authors) = &filters.authors {
        if !authors.is_empty() {
            let authors_lower: HashSet<String> = authors.iter().map(|a| a.to_lowercase()).collect();
            let paper_authors: HashSet<String> = paper
                .metadata
                .authors
                .iter()
                .map(|a| a.to_lowercase())
                .collect();
            if paper_authors.is_empty() || authors_lower.is_disjoint(&paper_authors) {
                return false;
            }
        }
    }

    if let Some(date_from) = parse_year(&filters.date_from) {
        if paper.metadata.year.map(|y| y as i32).unwrap_or(i32::MIN) < date_from {
            return false;
        }
    }

    if let Some(date_to) = parse_year(&filters.date_to) {
        if paper.metadata.year.map(|y| y as i32).unwrap_or(i32::MAX) > date_to {
            return false;
        }
    }

    if let Some(status_filters) = &filters.status {
        if !status_filters.is_empty() {
            let status = derive_status(paper);
            if !status_filters
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&status))
            {
                return false;
            }
        }
    }

    if let Some(tags) = &filters.tags {
        if !tags.is_empty() {
            let tags_lower: HashSet<String> = tags.iter().map(|t| t.to_lowercase()).collect();
            let paper_tags: HashSet<String> = paper
                .metadata
                .keywords
                .iter()
                .map(|t| t.to_lowercase())
                .collect();
            if paper_tags.is_empty() || tags_lower.is_disjoint(&paper_tags) {
                return false;
            }
        }
    }

    true
}

fn parse_year(value: &Option<String>) -> Option<i32> {
    value.as_ref().and_then(|year| year.parse::<i32>().ok())
}

fn derive_status(paper: &Paper) -> String {
    if paper
        .metadata
        .venue
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        "published".to_string()
    } else {
        "draft".to_string()
    }
}

fn derive_domains(paper: &Paper) -> Vec<String> {
    if paper.metadata.keywords.is_empty() {
        return vec!["unspecified".to_string()];
    }
    paper
        .metadata
        .keywords
        .iter()
        .map(|k| k.to_lowercase())
        .collect()
}

fn sort_candidates(candidates: &mut [RankedCandidate], sort: &SearchSort) {
    match sort {
        SearchSort::Relevance => {
            candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        }
        SearchSort::DateDescending => {
            candidates.sort_by(|a, b| {
                b.year
                    .cmp(&a.year)
                    .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal))
            });
        }
        SearchSort::DateAscending => {
            candidates.sort_by(|a, b| {
                a.year
                    .cmp(&b.year)
                    .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal))
            });
        }
        SearchSort::Title => {
            candidates.sort_by(|a, b| a.title.cmp(&b.title));
        }
    }
}

fn build_facets_from_candidates(candidates: &[RankedCandidate]) -> SearchFacets {
    let mut domain_counts: HashMap<String, usize> = HashMap::new();
    let mut author_counts: HashMap<String, usize> = HashMap::new();
    let mut year_counts: HashMap<String, usize> = HashMap::new();
    let mut status_counts: HashMap<String, usize> = HashMap::new();

    for candidate in candidates {
        for domain in &candidate.domains {
            *domain_counts.entry(domain.clone()).or_insert(0) += 1;
        }
        for author in &candidate.authors {
            *author_counts.entry(author.clone()).or_insert(0) += 1;
        }
        if let Some(year) = candidate.year {
            *year_counts.entry(year.to_string()).or_insert(0) += 1;
        }
        *status_counts.entry(candidate.status.clone()).or_insert(0) += 1;
    }

    SearchFacets {
        domains: to_sorted_facets(domain_counts),
        authors: to_sorted_facets(author_counts),
        years: to_sorted_facets(year_counts),
        statuses: to_sorted_facets(status_counts),
    }
}

fn build_facets_for_dataset(dataset: &[Paper]) -> SearchFacets {
    let mut domain_counts: HashMap<String, usize> = HashMap::new();
    let mut author_counts: HashMap<String, usize> = HashMap::new();
    let mut year_counts: HashMap<String, usize> = HashMap::new();
    let mut status_counts: HashMap<String, usize> = HashMap::new();

    for paper in dataset {
        for domain in derive_domains(paper) {
            *domain_counts.entry(domain).or_insert(0) += 1;
        }
        for author in &paper.metadata.authors {
            *author_counts.entry(author.clone()).or_insert(0) += 1;
        }
        if let Some(year) = paper.metadata.year {
            *year_counts.entry(year.to_string()).or_insert(0) += 1;
        }
        *status_counts.entry(derive_status(paper)).or_insert(0) += 1;
    }

    SearchFacets {
        domains: to_sorted_facets(domain_counts),
        authors: to_sorted_facets(author_counts),
        years: to_sorted_facets(year_counts),
        statuses: to_sorted_facets(status_counts),
    }
}

fn to_sorted_facets(map: HashMap<String, usize>) -> Vec<FacetItem> {
    let mut facets: Vec<FacetItem> = map
        .into_iter()
        .map(|(value, count)| FacetItem { value, count })
        .collect();

    facets.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
    facets
}

fn build_suggestions(
    dataset: &[Paper],
    query: &str,
    max_suggestions: usize,
) -> Vec<SearchSuggestion> {
    let normalized_query = query.trim().to_lowercase();
    let mut frequency: HashMap<String, usize> = HashMap::new();

    for paper in dataset {
        for segment in &paper.segments {
            for token in tokenize(&segment.content) {
                *frequency.entry(token).or_insert(0) += 1;
            }
        }

        for keyword in &paper.metadata.keywords {
            *frequency.entry(keyword.to_lowercase()).or_insert(0) += 3;
        }
    }

    let mut suggestions: Vec<(String, usize)> = frequency
        .into_iter()
        .filter(|(token, _)| {
            if normalized_query.is_empty() {
                true
            } else {
                token.starts_with(&normalized_query)
            }
        })
        .collect();

    suggestions.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    suggestions
        .into_iter()
        .take(max_suggestions)
        .map(|(token, count)| {
            if normalized_query.is_empty() {
                SearchSuggestion { text: token, count }
            } else {
                let suggestion_text = if token == normalized_query {
                    token
                } else {
                    format!("{} {}", normalized_query, token)
                };
                SearchSuggestion {
                    text: suggestion_text,
                    count,
                }
            }
        })
        .collect()
}

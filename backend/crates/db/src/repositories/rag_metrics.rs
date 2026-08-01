//! Observability for pgvector RAG retrieval quality (Story 84.5).
//!
//! The pgvector RAG store (`document_embeddings`) is the retrieval half of the
//! Enhanced-Chat RAG flow. Before feature adoption grows we want to see how
//! healthy retrieval is in production without shipping the whole answer through
//! a log line. This module emits three quality signals per retrieval:
//!
//! * **retrieval latency** — `rag_retrieval_duration_seconds` (histogram),
//!   labelled by `backend` (`pgvector` native index vs `fallback` app-level
//!   cosine scan) so a slow fallback path is visible.
//! * **top-k relevance** — `rag_retrieval_relevance_score` (histogram): the
//!   cosine similarity of every returned chunk, giving the relevance
//!   distribution of what actually reached the model.
//! * **empty-result rate** — `rag_retrieval_total` and
//!   `rag_retrieval_empty_total` (counters): the ratio
//!   `empty / total` is the empty-result rate. `rag_retrieval_results`
//!   (histogram) records the per-call result count for finer analysis.
//!
//! Metrics are emitted through the `metrics` facade rather than a concrete
//! exporter, so the db crate stays exporter-agnostic: any binary that installs
//! a recorder (api-server's Prometheus exporter) captures them, and when no
//! recorder is installed (unit tests, one-off tooling) the macros are cheap
//! no-ops.

use std::sync::Once;
use std::time::Duration;

/// Histogram: pgvector RAG retrieval latency, in seconds.
pub const METRIC_DURATION: &str = "rag_retrieval_duration_seconds";
/// Histogram: cosine similarity of each returned chunk (top-k relevance).
pub const METRIC_RELEVANCE: &str = "rag_retrieval_relevance_score";
/// Histogram: number of chunks returned per retrieval.
pub const METRIC_RESULTS: &str = "rag_retrieval_results";
/// Counter: total RAG retrievals.
pub const METRIC_TOTAL: &str = "rag_retrieval_total";
/// Counter: RAG retrievals that returned zero chunks (empty-result numerator).
pub const METRIC_EMPTY: &str = "rag_retrieval_empty_total";

/// Label value: native pgvector index path (`search_similar_documents`).
pub const BACKEND_PGVECTOR: &str = "pgvector";
/// Label value: application-level cosine-similarity fallback scan.
pub const BACKEND_FALLBACK: &str = "fallback";

static DESCRIBE: Once = Once::new();

/// Register metric descriptions once per process. Idempotent and safe to call
/// on every retrieval; the `Once` guard makes all but the first call a no-op.
fn describe_once() {
    DESCRIBE.call_once(|| {
        metrics::describe_histogram!(
            METRIC_DURATION,
            "pgvector RAG retrieval latency in seconds, labelled by backend (pgvector|fallback)"
        );
        metrics::describe_histogram!(
            METRIC_RELEVANCE,
            "Cosine similarity of each returned RAG chunk (top-k relevance distribution)"
        );
        metrics::describe_histogram!(
            METRIC_RESULTS,
            "Number of chunks returned per RAG retrieval"
        );
        metrics::describe_counter!(
            METRIC_TOTAL,
            "Total pgvector RAG retrievals, labelled by backend"
        );
        metrics::describe_counter!(
            METRIC_EMPTY,
            "RAG retrievals that returned zero chunks (empty-result rate numerator)"
        );
    });
}

/// A recorder-free summary of one retrieval's outcome.
///
/// Split out from metric emission so the classification logic (empty vs
/// non-empty, top score) can be unit-tested without installing a global
/// `metrics` recorder.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalObservation {
    /// Which retrieval path served the query.
    pub backend: &'static str,
    /// Number of chunks returned.
    pub result_count: usize,
    /// True when no chunk cleared the similarity threshold.
    pub is_empty: bool,
    /// Highest similarity among returned chunks, if any.
    pub top_score: Option<f64>,
}

impl RetrievalObservation {
    /// Summarise a set of returned similarity scores.
    pub fn new(backend: &'static str, scores: &[f64]) -> Self {
        let top_score = scores.iter().copied().fold(None, |acc, s| match acc {
            Some(m) if m >= s => Some(m),
            _ => Some(s),
        });
        Self {
            backend,
            result_count: scores.len(),
            is_empty: scores.is_empty(),
            top_score,
        }
    }
}

/// Record one RAG retrieval's latency and relevance outcome.
///
/// `backend` should be [`BACKEND_PGVECTOR`] or [`BACKEND_FALLBACK`].
/// `scores` are the cosine similarities of the returned chunks (empty slice ==
/// empty result).
pub fn record_retrieval(backend: &'static str, elapsed: Duration, scores: &[f64]) {
    describe_once();
    let obs = RetrievalObservation::new(backend, scores);

    metrics::histogram!(METRIC_DURATION, "backend" => backend).record(elapsed.as_secs_f64());
    metrics::counter!(METRIC_TOTAL, "backend" => backend).increment(1);
    metrics::histogram!(METRIC_RESULTS, "backend" => backend).record(obs.result_count as f64);

    if obs.is_empty {
        metrics::counter!(METRIC_EMPTY, "backend" => backend).increment(1);
    } else {
        for &score in scores {
            metrics::histogram!(METRIC_RELEVANCE, "backend" => backend).record(score);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_empty_scores() {
        let obs = RetrievalObservation::new(BACKEND_PGVECTOR, &[]);
        assert_eq!(obs.backend, "pgvector");
        assert_eq!(obs.result_count, 0);
        assert!(obs.is_empty);
        assert_eq!(obs.top_score, None);
    }

    #[test]
    fn observation_tracks_count_and_top_score() {
        let obs = RetrievalObservation::new(BACKEND_FALLBACK, &[0.62, 0.91, 0.75]);
        assert_eq!(obs.backend, "fallback");
        assert_eq!(obs.result_count, 3);
        assert!(!obs.is_empty);
        assert_eq!(obs.top_score, Some(0.91));
    }

    #[test]
    fn observation_single_score_is_top() {
        let obs = RetrievalObservation::new(BACKEND_PGVECTOR, &[0.5]);
        assert!(!obs.is_empty);
        assert_eq!(obs.top_score, Some(0.5));
    }

    #[test]
    fn record_retrieval_is_noop_without_recorder() {
        // No global recorder is installed in unit tests; the facade macros must
        // remain cheap no-ops and never panic on either the empty or populated
        // path.
        record_retrieval(BACKEND_PGVECTOR, Duration::from_millis(12), &[0.8, 0.7]);
        record_retrieval(BACKEND_FALLBACK, Duration::from_millis(40), &[]);
    }
}

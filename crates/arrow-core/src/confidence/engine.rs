use crate::{registry::loader::Registries, types::message::Confidence};

#[derive(Debug, Clone)]
pub struct SourceObservation {
    pub source_id: String,
    pub source_type: String,
    pub score: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfidenceEngineResult {
    Computed { composite_numeric: f64, categorical: Confidence },
    Divergence { source_a: String, source_b: String, metric: String, score_a: f64, score_b: f64, delta: f64, threshold: f64 },
}

/// Confidence Engine, Part 9.
pub fn compute(observations: &[SourceObservation], divergence_threshold: f64, _registries: &Registries) -> ConfidenceEngineResult {
    for i in 0..observations.len() {
        for j in (i + 1)..observations.len() {
            let delta = (observations[i].score - observations[j].score).abs();
            if delta > divergence_threshold {
                return ConfidenceEngineResult::Divergence {
                    source_a: observations[i].source_id.clone(),
                    source_b: observations[j].source_id.clone(),
                    metric: "score".to_owned(),
                    score_a: observations[i].score,
                    score_b: observations[j].score,
                    delta,
                    threshold: divergence_threshold,
                };
            }
        }
    }
    let total_weight: f64 = observations.iter().map(|o| o.weight).sum();
    let composite = if observations.is_empty() || total_weight <= 0.0 { 0.0 } else { observations.iter().map(|o| o.score * o.weight).sum::<f64>() / total_weight };
    let categorical = if composite >= 0.95 { Confidence::A } else if composite >= 0.80 { Confidence::B } else if composite >= 0.60 { Confidence::C } else if composite >= 0.40 { Confidence::D } else { Confidence::E };
    ConfidenceEngineResult::Computed { composite_numeric: composite, categorical }
}

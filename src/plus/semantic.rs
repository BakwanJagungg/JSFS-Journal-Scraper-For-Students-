//! Hybrid Semantic Reranker & Citation Impact Scoring for JSFS+ Workstation:
//! 1. Offline Pure Rust Semantic Vector (char 3-gram + word bigram dense L2-normalized projection)
//! 2. Optional Ollama GPU Embedding Bridge (`/api/embeddings` for `nomic-embed-text` / `bge-m3` etc.)
//! 3. OpenAlex Citation Velocity boost (`cited_by_count` / `works_count`)

use crate::config::OLLAMA_BASE_URL;
use crate::models::{JournalRecord, ManuscriptMeta};
use std::time::Duration;

pub const SEMANTIC_DIM: usize = 256;

#[inline]
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn add_projected_feature(vec: &mut [f32; SEMANTIC_DIM], key: &[u8], weight: f32) {
    let h = fnv1a_64(key);
    let idx = (h as usize) & (SEMANTIC_DIM - 1);
    let sign = if ((h >> 63) & 1) == 0 { 1.0 } else { -1.0 };
    vec[idx] += sign * weight;
}

/// Project arbitrary bilingual text into an L2-normalized 256-D pure-Rust semantic vector
/// combining word unigrams, word bigrams, and character 3-grams.
pub fn embed_text_local(text: &str) -> [f32; SEMANTIC_DIM] {
    let mut vec = [0.0_f32; SEMANTIC_DIM];
    let normalized: String = text
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();

    let tokens: Vec<&str> = normalized
        .split_whitespace()
        .filter(|w| w.len() >= 2)
        .take(350)
        .collect();

    for (i, &tok) in tokens.iter().enumerate() {
        let stemmed = crate::parser::doc_matcher::stem_academic_word(tok);
        add_projected_feature(&mut vec, stemmed.as_bytes(), 1.25);

        if i + 1 < tokens.len() {
            let next_stem = crate::parser::doc_matcher::stem_academic_word(tokens[i + 1]);
            let bigram = format!("{stemmed}_{next_stem}");
            add_projected_feature(&mut vec, bigram.as_bytes(), 1.6);
        }

        let b = stemmed.as_bytes();
        if b.len() >= 3 {
            for win in b.windows(3) {
                add_projected_feature(&mut vec, win, 0.45);
            }
        }
    }

    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for v in &mut vec {
            *v /= norm;
        }
    }
    vec
}

/// Compute clamped cosine similarity (`0.0..=1.0`) between two embedding slices.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na * nb).sqrt();
    if denom <= 1e-8 {
        0.0
    } else {
        (dot / denom).clamp(0.0, 1.0)
    }
}

/// Compute average citations per work (`Citation Velocity`) from OpenAlex counts.
pub fn compute_citation_velocity(cited_by: Option<u64>, works: Option<u64>) -> Option<f64> {
    let cited = cited_by?;
    let w = works.filter(|&w| w > 0)?;
    Some((cited as f64) / (w as f64))
}

async fn detect_ollama_embed_model(client: &reqwest::Client) -> Option<String> {
    let resp = client
        .get(format!("{OLLAMA_BASE_URL}/api/tags"))
        .timeout(Duration::from_millis(750))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let val: serde_json::Value = resp.json().await.ok()?;
    let models = val.get("models")?.as_array()?;
    let preferred = [
        "nomic-embed-text",
        "bge-m3",
        "mxbai-embed-large",
        "snowflake-arctic-embed",
        "all-minilm",
    ];
    for pref in preferred {
        for m in models {
            if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                if name.to_ascii_lowercase().contains(pref) {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

async fn fetch_ollama_embedding(
    client: &reqwest::Client,
    model: &str,
    text: &str,
) -> Option<Vec<f32>> {
    let trimmed: String = text.chars().take(1800).collect();
    let payload = serde_json::json!({
        "model": model,
        "prompt": trimmed,
    });
    let resp = client
        .post(format!("{OLLAMA_BASE_URL}/api/embeddings"))
        .json(&payload)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let val: serde_json::Value = resp.json().await.ok()?;
    let arr = val.get("embedding")?.as_array()?;
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        out.push(v.as_f64()? as f32);
    }
    Some(out)
}

/// Rerank candidate journals using Pure Rust 256-D semantic similarity, optional GPU-accelerated
/// Ollama vector embeddings, and OpenAlex Citation Velocity impact boost.
pub async fn rerank_candidates(
    client: &reqwest::Client,
    records: &mut [JournalRecord],
    keywords: &[String],
    manuscript: Option<&ManuscriptMeta>,
) -> Option<String> {
    if records.is_empty() {
        return None;
    }

    let query_text = match manuscript {
        Some(ms) => format!(
            "{} {} {} {}",
            keywords.join(" "),
            ms.extracted_title,
            ms.abstract_snippet,
            ms.detected_keywords.join(" ")
        ),
        None => keywords.join(" "),
    };
    let query_local = embed_text_local(&query_text);

    // Opportunistic Ollama embedding bridge (fails fast in <750ms when offline)
    let embed_model = detect_ollama_embed_model(client).await;
    let query_ollama = if let Some(ref model) = embed_model {
        fetch_ollama_embedding(client, model, &query_text).await
    } else {
        None
    };

    // Keep Ollama GPU vector queries bounded to the top 24 BM25 candidates by current score
    let mut top_indices: Vec<usize> = (0..records.len()).collect();
    top_indices.sort_by(|&a, &b| records[b].match_score.total_cmp(&records[a].match_score));
    top_indices.truncate(24);

    let mut ollama_scores: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
    if let (Some(ref model), Some(ref q_vec)) = (&embed_model, &query_ollama) {
        let futs = top_indices.iter().map(|&idx| {
            let doc_txt = format!(
                "{} {} {}",
                records[idx].title, records[idx].subject_area, records[idx].focus_and_scope
            );
            async move {
                let emb = fetch_ollama_embedding(client, model, &doc_txt).await?;
                Some((idx, cosine_similarity(q_vec, &emb)))
            }
        });
        for item in futures_util::future::join_all(futs)
            .await
            .into_iter()
            .flatten()
        {
            ollama_scores.insert(item.0, item.1);
        }
    }

    for (idx, rec) in records.iter_mut().enumerate() {
        let doc_text = format!("{} {} {}", rec.title, rec.subject_area, rec.focus_and_scope);
        let doc_local = embed_text_local(&doc_text);
        let local_sim = cosine_similarity(&query_local, &doc_local);

        // Citation velocity boost: up to +4.0 pts for journals with sustained per-paper impact
        let impact_boost = match compute_citation_velocity(rec.cited_by_count, rec.works_count) {
            Some(vel) if vel > 0.0 => (vel.ln_1p() * 1.4).min(4.0),
            _ => 0.0,
        };

        let neural_sim = ollama_scores.get(&idx).copied();
        let blended = if let Some(nsim) = neural_sim {
            let sem_pct = ((local_sim * 0.35 + nsim * 0.65) * 100.0) as f64;
            (rec.match_score * 0.60) + (sem_pct * 0.40) + impact_boost
        } else {
            let sem_pct = (local_sim * 100.0) as f64;
            (rec.match_score * 0.78) + (sem_pct * 0.22) + impact_boost
        };
        rec.match_score = blended.clamp(0.0, 100.0);

        if rec.match_score >= 70.0 {
            rec.match_label = "Sangat Cocok".to_string();
        } else if rec.match_score >= 45.0 {
            rec.match_label = "Cukup Cocok".to_string();
        }

        if let Some(nsim) = neural_sim {
            let tag = format!("Semantik GPU {:.0}%", nsim * 100.0);
            if !rec.match_reasons.iter().any(|r| r.contains("Semantik")) {
                rec.match_reasons.push(tag);
            }
        } else if local_sim >= 0.25 && !rec.match_reasons.iter().any(|r| r.contains("Semantik")) {
            rec.match_reasons
                .push(format!("Vektor Semantik {:.0}%", local_sim * 100.0));
        }

        if let Some(vel) = compute_citation_velocity(rec.cited_by_count, rec.works_count) {
            let vel_tag = format!("Impact {:.1} sitasi/artikel", vel);
            if !rec.match_reasons.iter().any(|r| r.contains("Impact")) {
                rec.match_reasons.push(vel_tag);
            }
        }
    }

    embed_model
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_rust_semantic_cosine_and_citation_velocity() {
        let v_a = embed_text_local(
            "machine learning sentiment analysis classification for student academic journals",
        );
        let v_b = embed_text_local(
            "sentiment classification with machine learning models in academic text analytics",
        );
        let v_c = embed_text_local(
            "pediatric dental anesthesia protocols and clinical surgery care management",
        );

        let sim_ab = cosine_similarity(&v_a, &v_b);
        let sim_ac = cosine_similarity(&v_a, &v_c);
        assert!(
            sim_ab > sim_ac + 0.15,
            "related topic cosine ({sim_ab:.3}) must substantially exceed unrelated ({sim_ac:.3})"
        );

        assert_eq!(compute_citation_velocity(Some(500), Some(100)), Some(5.0));
        assert_eq!(compute_citation_velocity(Some(10), Some(0)), None);
    }
}

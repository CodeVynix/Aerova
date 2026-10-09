//! Aerova Search v1: in-memory inverted index + TF scoring (Phase 7).
//! tantivy migration later; std-only keeps Lumora port trivial.

use std::collections::HashMap;

/// Document id.
pub type DocId = usize;

/// Tiny search index.
#[derive(Debug, Default)]
pub struct Index {
    docs: Vec<(String, String)>,
    postings: HashMap<String, Vec<(DocId, u32)>>,
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .map(ToString::to_string)
        .collect()
}

impl Index {
    /// Create empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add document with title + body, returns id.
    pub fn add(&mut self, title: &str, body: &str) -> DocId {
        let id = self.docs.len();
        self.docs.push((title.to_string(), body.to_string()));
        let mut counts: HashMap<String, u32> = HashMap::new();
        for w in tokenize(&format!("{title} {body}")) {
            *counts.entry(w).or_insert(0) += 1;
        }
        for (w, c) in counts {
            self.postings.entry(w).or_default().push((id, c));
        }
        id
    }

    /// Search, ranked by term frequency. Returns `(id, score)`.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<(DocId, u32)> {
        let mut scores: HashMap<DocId, u32> = HashMap::new();
        for w in tokenize(query) {
            if let Some(list) = self.postings.get(&w) {
                for (id, c) in list {
                    *scores.entry(*id).or_insert(0) += *c;
                }
            }
        }
        let mut v: Vec<(DocId, u32)> = scores.into_iter().collect();
        v.sort_by_key(|b| std::cmp::Reverse(b.1));
        v.truncate(limit);
        v
    }

    /// Get document.
    #[must_use]
    pub fn get(&self, id: DocId) -> Option<(&str, &str)> {
        self.docs.get(id).map(|(t, b)| (t.as_str(), b.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_title_match() {
        let mut idx = Index::new();
        idx.add("Aerova browser", "fast private engine");
        idx.add("Cooking soup", "carrots onions");
        let r = idx.search("aerova", 10);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, 0);
    }
}

//! Aerova Search v2: BM25 + title boost, std-only file persistence.
//! tantivy migration later; BM25 keeps Lumora port trivial.
//!
//! Scoring: `sum IDF * (tf*(k1+1))/(tf+k1*(1-b+b*dl/avgdl))`
//! with `k1=1.2`, `b=0.75`, Lucene-style IDF, `tf = body + 3*title`,
//! `dl = body_len + 3*title_len`.

use std::collections::HashMap;
use std::path::Path;

/// Document id.
pub type DocId = usize;

/// Title terms count this much per occurrence.
const TITLE_BOOST: f32 = 3.0;
/// BM25 saturation.
const K1: f32 = 1.2;
/// BM25 length normalization.
const B: f32 = 0.75;

/// Tiny search index.
#[derive(Debug, Default)]
pub struct Index {
    docs: Vec<(String, String)>,
    postings: HashMap<String, Vec<(DocId, f32)>>,
    doc_lens: Vec<f32>,
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .map(ToString::to_string)
        .collect()
}

fn idf(n_docs: usize, doc_freq: usize) -> f32 {
    (((n_docs as f32 - doc_freq as f32 + 0.5) / (doc_freq as f32 + 0.5)) + 1.0).ln()
}

impl Index {
    /// Create empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of documents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// True if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Add document with title + body, returns id.
    pub fn add(&mut self, title: &str, body: &str) -> DocId {
        let id = self.docs.len();
        let title_toks = tokenize(title);
        let body_toks = tokenize(body);
        let dl = body_toks.len() as f32 + TITLE_BOOST * title_toks.len() as f32;
        self.docs.push((title.to_string(), body.to_string()));
        self.doc_lens.push(dl);
        let mut counts: HashMap<String, (f32, f32)> = HashMap::new();
        for w in body_toks {
            let e = counts.entry(w).or_insert((0.0, 0.0));
            e.0 += 1.0;
        }
        for w in title_toks {
            let e = counts.entry(w).or_insert((0.0, 0.0));
            e.1 += 1.0;
        }
        for (w, (body_c, title_c)) in counts {
            let tf = body_c + TITLE_BOOST * title_c;
            self.postings.entry(w).or_default().push((id, tf));
        }
        id
    }

    /// Search, ranked by BM25. Returns `(id, score)` descending.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<(DocId, f32)> {
        let n = self.docs.len();
        if n == 0 {
            return Vec::new();
        }
        let avgdl: f32 = self.doc_lens.iter().sum::<f32>() / n as f32;
        let mut scores: HashMap<DocId, f32> = HashMap::new();
        for w in tokenize(query) {
            let Some(list) = self.postings.get(&w) else {
                continue;
            };
            let weight = idf(n, list.len());
            for (id, tf) in list {
                let dl = self.doc_lens[*id];
                let norm = K1 * (1.0 - B + B * dl / avgdl.max(1.0));
                let s = weight * (*tf * (K1 + 1.0)) / (*tf + norm);
                *scores.entry(*id).or_insert(0.0) += s;
            }
        }
        let mut v: Vec<(DocId, f32)> = scores.into_iter().collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(limit);
        v
    }

    /// Get document.
    #[must_use]
    pub fn get(&self, id: DocId) -> Option<(&str, &str)> {
        self.docs.get(id).map(|(t, b)| (t.as_str(), b.as_str()))
    }

    /// Save to file. Format: AEROVA-IDX-1 header + len-prefixed records.
    /// # Errors
    /// Returns IO error if write fails.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut out = Vec::new();
        out.extend_from_slice(b"AEROVA-IDX-1\n");
        out.extend_from_slice(format!("{}\n", self.docs.len()).as_bytes());
        for (title, body) in &self.docs {
            out.extend_from_slice(format!("TITLE {}\n", title.len()).as_bytes());
            out.extend_from_slice(title.as_bytes());
            out.push(b'\n');
            out.extend_from_slice(format!("BODY {}\n", body.len()).as_bytes());
            out.extend_from_slice(body.as_bytes());
            out.push(b'\n');
        }
        std::fs::write(path, out)
    }

    /// Load from file written by [`Index::save`]. Rebuilds postings + BM25 stats.
    /// # Errors
    /// Returns IO error if read/parse fails.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let data = std::fs::read(path)?;
        let mut idx = Self::new();
        let mut pos = 0;
        let header = b"AEROVA-IDX-1\n";
        if data.len() < header.len() || &data[..header.len()] != header {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad header",
            ));
        }
        pos += header.len();
        let line_end = data[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad count"))?;
        let count: usize = std::str::from_utf8(&data[pos..pos + line_end])
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad count"))?
            .trim()
            .parse()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad count"))?;
        pos += line_end + 1;
        for _ in 0..count {
            let (title, next) = read_record(&data, pos, "TITLE")?;
            pos = next;
            let (body, next) = read_record(&data, pos, "BODY")?;
            pos = next;
            idx.add(&title, &body);
        }
        return Ok(idx);

        fn read_record(
            data: &[u8],
            mut pos: usize,
            kind: &str,
        ) -> std::io::Result<(String, usize)> {
            let prefix = format!("{kind} ");
            let line_end = data
                .get(pos..)
                .and_then(|s| s.iter().position(|&b| b == b'\n'))
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "bad record")
                })?;
            let line = std::str::from_utf8(&data[pos..pos + line_end])
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad record"))?;
            let len: usize = line
                .strip_prefix(&prefix)
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad record"))?
                .trim()
                .parse()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad record"))?;
            pos += line_end + 1;
            let end = pos
                .checked_add(len)
                .filter(|&e| e <= data.len())
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "truncated"))?;
            let s = std::str::from_utf8(&data[pos..end])
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad utf8"))?
                .to_string();
            pos = end + 1;
            Ok((s, pos))
        }
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

    #[test]
    fn title_boost_beats_body() {
        let mut idx = Index::new();
        idx.add("soup recipes", "unrelated filler words here");
        idx.add("dinner ideas", "soup soup plus filler words here");
        let r = idx.search("soup", 10);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, 0, "title match should outrank body double");
    }

    #[test]
    fn rare_term_outranks_common() {
        let mut idx = Index::new();
        idx.add("a", "common words here");
        idx.add("b", "common words here");
        idx.add("c", "common words here rare");
        let r = idx.search("common rare", 10);
        assert_eq!(r[0].0, 2);
    }

    #[test]
    fn roundtrip_save_load() {
        let mut idx = Index::new();
        idx.add("Aerova home", "Welcome fast engine\nsecond line");
        idx.add("Lumora OS", "desktop ports");
        let dir = std::env::temp_dir().join(format!("aerova-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("idx.db");
        idx.save(&path).unwrap();
        let loaded = Index::load(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.search("aerova", 10).len(), 1);
        assert_eq!(loaded.get(0).unwrap().0, "Aerova home");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

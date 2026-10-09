//! Aerova shell v0.3: CLI with crawl + search + isolated tabs demo.

use std::path::{Path, PathBuf};

fn usage() -> &'static str {
    "usage: aerova <open <url|file> [--shot out.ppm] | crawl <url>... [--db FILE] | search <query> [--db FILE] | tabs demo | js <code> | home>"
}

fn parse_db(args: &[String]) -> Option<PathBuf> {
    let mut i = 0;
    while i + 1 < args.len() {
        if args[i] == "--db" {
            return Some(PathBuf::from(&args[i + 1]));
        }
        i += 1;
    }
    None
}

fn strip_db(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--db" {
            i += 2;
        } else {
            out.push(args[i].clone());
            i += 1;
        }
    }
    out
}

fn load_or_new(path: Option<&Path>) -> aerova_search::Index {
    if let Some(p) = path {
        if p.exists() {
            match aerova_search::Index::load(p) {
                Ok(idx) => return idx,
                Err(e) => eprintln!("warn: cannot load {}: {e}, starting new", p.display()),
            }
        }
    }
    aerova_search::Index::new()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{usage}", usage = usage());
        std::process::exit(2);
    }
    let policy = aerova_engine::IsolationPolicy::per_tab_thread();
    match args[0].as_str() {
        "open" => {
            if args.len() < 2 {
                eprintln!("{usage}", usage = usage());
                std::process::exit(2);
            }
            let target = &args[1];
            let url = if target.contains("://") {
                target.clone()
            } else {
                format!("file://{target}")
            };
            match aerova_engine::render_isolated(&url, &policy) {
                Ok((text, fb)) => {
                    println!("{text}");
                    if args.len() == 4 && args[2] == "--shot" {
                        let out = PathBuf::from(&args[3]);
                        if let Err(e) = fb.save_ppm(&out) {
                            eprintln!("shot failed: {e}");
                            std::process::exit(1);
                        }
                        eprintln!("saved {}", out.display());
                    }
                }
                Err(e) => {
                    eprintln!("open failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "crawl" => {
            let rest = strip_db(&args[1..]);
            let db = parse_db(&args[1..]);
            if rest.is_empty() {
                eprintln!("{usage}", usage = usage());
                std::process::exit(2);
            }
            let mut idx = load_or_new(db.as_deref());
            for url in &rest {
                match aerova_engine::index_url(&mut idx, url) {
                    Ok(id) => {
                        let title = idx.get(id).map_or(url.clone(), |(t, _)| t.to_string());
                        println!("indexed {id} {title}");
                    }
                    Err(e) => {
                        eprintln!("crawl failed {url}: {e}");
                        std::process::exit(1);
                    }
                }
            }
            if let Some(p) = db {
                if let Err(e) = idx.save(&p) {
                    eprintln!("save failed: {e}");
                    std::process::exit(1);
                }
                eprintln!("saved {} docs to {}", idx.len(), p.display());
            } else {
                eprintln!("indexed {} docs (no --db, in-memory only)", idx.len());
            }
        }
        "search" => {
            let rest = strip_db(&args[1..]);
            let db = parse_db(&args[1..]);
            if rest.is_empty() {
                eprintln!("{usage}", usage = usage());
                std::process::exit(2);
            }
            let q = rest.join(" ");
            let mut idx = load_or_new(db.as_deref());
            if idx.is_empty() {
                idx.add("Aerova home", "Welcome to Aerova fast private engine");
                idx.add(
                    "Lumora OS",
                    "from scratch desktop OS ports chromium firefox",
                );
            }
            for (id, score) in idx.search(&q, 10) {
                if let Some((t, _)) = idx.get(id) {
                    println!("{score} {t}");
                }
            }
        }
        "tabs" => {
            let mut m = aerova_engine::TabManager::new();
            let a = m.new_tab("aerova://home");
            let b = m.new_tab("aerova://home");
            m.navigate(b, "bogus://nope");
            for (id, url) in m.list() {
                match aerova_engine::render_isolated(&url, &policy) {
                    Ok((text, _)) => println!("tab {id} ok: {text}"),
                    Err(e) => println!("tab {id} isolated failure (tab survives): {e}"),
                }
            }
            let _ = (a, b);
        }
        "js" => {
            let code = args[1..].join(" ");
            match aerova_js::eval(&code) {
                Ok(v) => println!("{v}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        "home" => match aerova_engine::render_isolated("aerova://home", &policy) {
            Ok((text, _)) => println!("{text}"),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("{usage}", usage = usage());
            std::process::exit(2);
        }
    }
}

//! Aerova shell v0.1: CLI only (GUI winit/wgpu after local render is solid).

use std::path::PathBuf;

fn usage() -> &'static str {
    "usage: aerova <open <url|file> [--shot out.ppm] | search <query> | js <code> | home>"
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{usage}", usage = usage());
        std::process::exit(2);
    }
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
            match aerova_engine::render_url(&url) {
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
        "search" => {
            let q = args[1..].join(" ");
            let mut idx = aerova_search::Index::new();
            idx.add("Aerova home", "Welcome to Aerova fast private engine");
            idx.add(
                "Lumora OS",
                "from scratch desktop OS ports chromium firefox",
            );
            for (id, score) in idx.search(&q, 10) {
                if let Some((t, _)) = idx.get(id) {
                    println!("{score} {t}");
                }
            }
        }
        "js" => {
            let code = args[1..].join(" ");
            match aerova_js::eval_stub(&code) {
                Ok(v) => println!("{v}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        "home" => match aerova_engine::render_url("aerova://home") {
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

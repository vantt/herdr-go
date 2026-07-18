//! Guarantees `static/` exists before `RustEmbed`'s derive macro scans it, so
//! `cargo build`/`test`/`clippy` never fail on a fresh checkout where
//! `npm run bundle` hasn't produced the web UI yet (`static/` is gitignored).

fn main() {
    std::fs::create_dir_all("static").expect("create static/ dir for embedding");
    println!("cargo:rerun-if-changed=static");
}

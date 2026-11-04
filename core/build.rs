use std::path::{Path, PathBuf};

fn main() {
    println!(
        "cargo::rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
    embed_client();
}

/// Embeds Builder's chat client (the app's `npm run build:builder` output,
/// copied to `client/` by CI or `make client`) so the installer needs no
/// running app. Without it, the page loads the chat from `client_url`.
fn embed_client() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = std::env::var("VOIPAPPZ_BUILDER_CLIENT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest.join("../client"));
    println!("cargo::rerun-if-env-changed=VOIPAPPZ_BUILDER_CLIENT_DIR");
    println!("cargo::rerun-if-changed={}", dir.display());

    let mut files = Vec::new();
    if dir.join("client.js").is_file() {
        collect(&dir, &dir, &mut files);
    }
    files.sort();
    let entries = files
        .iter()
        .map(|(rel, abs)| {
            println!("cargo::rerun-if-changed={}", abs.display());
            format!("    ({rel:?}, include_bytes!({:?})),\n", abs.display().to_string())
        })
        .collect::<String>();
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("client_files.rs");
    std::fs::write(
        out,
        format!("pub static CLIENT_FILES: &[(&str, &[u8])] = &[\n{entries}];\n"),
    )
    .unwrap();
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            files.push((rel, path.canonicalize().unwrap()));
        }
    }
}

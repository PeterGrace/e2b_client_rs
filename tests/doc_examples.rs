//! Every public type and every public async method has an `# Examples`
//! section in its rustdoc (CLM-0120).
//!
//! The test reads the crate's source, so it needs no network. "Public type"
//! means a `pub struct` or `pub enum` at the top level of a file whose name
//! `src/lib.rs` re-exports; "public async method" means any `pub async fn`
//! (crate-private helpers are `pub(crate)` and are skipped).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The names `lib.rs` re-exports with `pub use <path>;` or
/// `pub use <path>::{A, B};`.
///
/// # Arguments
///
/// * `lib` - the text of `src/lib.rs`.
///
/// # Returns
///
/// The re-exported names (modules too, which never match a type).
fn reexported_names(lib: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = lib;
    while let Some(start) = rest.find("pub use ") {
        let after = &rest[start + "pub use ".len()..];
        let end = after.find(';').expect("a `pub use` ends with `;`");
        let path = &after[..end];
        let items = match (path.find('{'), path.rfind('}')) {
            (Some(open), Some(close)) => &path[open + 1..close],
            _ => path
                .rsplit("::")
                .next()
                .expect("`rsplit` yields at least one item"),
        };
        names.extend(
            items
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
        );
        rest = &after[end..];
    }
    names
}

/// Every `.rs` file under `dir`, recursively.
///
/// # Arguments
///
/// * `dir` - the directory to walk.
/// * `out` - where the paths are collected.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read a source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The kind and name of the item a line declares, if it declares a public
/// type at the top level or a public async function at any depth.
///
/// # Arguments
///
/// * `line` - one source line.
///
/// # Returns
///
/// `Some(("type" | "async fn", name))`, or `None`.
fn declared_item(line: &str) -> Option<(&'static str, &str)> {
    let trimmed = line.trim_start();
    let (kind, rest) = if let Some(rest) = line.strip_prefix("pub struct ") {
        ("type", rest)
    } else if let Some(rest) = line.strip_prefix("pub enum ") {
        ("type", rest)
    } else {
        // `?` on an `Option` returns `None` early, like a guard clause.
        ("async fn", trimmed.strip_prefix("pub async fn ")?)
    };
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    Some((kind, &rest[..end]))
}

/// Whether the doc comment directly above line `item` has `# Examples`.
/// Attributes (`#[...]`) and plain `//` comments between the doc comment
/// and the item are skipped; a blank line or code ends the search.
///
/// # Arguments
///
/// * `lines` - the file's lines.
/// * `item` - the index of the declaring line.
fn has_examples(lines: &[&str], item: usize) -> bool {
    lines[..item]
        .iter()
        .rev()
        .map(|line| line.trim())
        // `//` also matches `///`, so one test covers doc and plain comments.
        .take_while(|line| line.starts_with("//") || line.starts_with("#["))
        .any(|line| line == "/// # Examples")
}

#[test]
fn reexported_names_reads_single_and_grouped_uses() {
    // Arrange
    let lib = concat!(
        "pub use client::Client;\n",
        "pub use error::{ConnectCode, Error,\n    Result};\n",
        "pub use secrecy;\n",
    );

    // Act
    let names = reexported_names(lib);

    // Assert
    let expected: BTreeSet<String> = ["Client", "ConnectCode", "Error", "Result", "secrecy"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(names, expected);
}

#[test]
fn has_examples_looks_through_attributes_only() {
    // Arrange
    let lines = [
        "/// A thing.",
        "///",
        "/// # Examples",
        "#[derive(Debug)]",
        "pub struct Thing;",
        "",
        "/// No example.",
        "pub struct Other;",
        "",
        "/// Commented.",
        "///",
        "/// # Examples",
        "// Why the derive.",
        "#[derive(Debug)]",
        "pub struct Commented;",
    ];

    // Act and Assert
    assert!(has_examples(&lines, 4));
    assert!(!has_examples(&lines, 7));
    assert!(has_examples(&lines, 14));
}

#[test]
fn every_public_type_and_async_method_has_an_example() {
    // Arrange
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let lib = fs::read_to_string(src.join("lib.rs")).expect("read src/lib.rs");
    let exported = reexported_names(&lib);
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    files.sort();

    // Act
    let mut missing = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).expect("read a source file");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some((kind, name)) = declared_item(line) else {
                continue;
            };
            let checked = kind == "async fn" || exported.contains(name);
            if checked && !has_examples(&lines, index) {
                let shown = file.strip_prefix(&src).unwrap_or(file);
                missing.push(format!(
                    "src/{}:{} {kind} {name}",
                    shown.display(),
                    index + 1
                ));
            }
        }
    }

    // Assert
    assert!(
        missing.is_empty(),
        "{} item(s) lack an `# Examples` section:\n{}",
        missing.len(),
        missing.join("\n")
    );
}

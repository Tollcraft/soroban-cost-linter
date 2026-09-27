use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug)]
struct LintEntry {
    name_snake: String,
    name_doc: String,
    level: String,
    description: String,
    category: Option<String>,
}

/// Read every `declare_lint!` block in `lib.rs`, plus the category each lint
/// picks up from `LINT_METADATA`.
///
/// A block is bounded by the brace that balances its opening `{` — see
/// [`matching_close_brace`] — so the payload lines of one lint can never spill
/// into the next construct.
fn parse_lib_rs(content: &str) -> Vec<LintEntry> {
    let lines: Vec<&str> = content.lines().collect();
    let mut entries: Vec<LintEntry> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.contains("declare_lint!") && trimmed.ends_with('{') {
            match block_end_line(&lines, i)
                .and_then(|end| parse_block_body(&lines[i + 1..end]).map(|body| (end, body)))
            {
                Some((end, (name_snake, level, desc))) => {
                    entries.push(LintEntry {
                        name_doc: name_snake.to_lowercase(),
                        name_snake,
                        level,
                        description: desc,
                        category: None,
                    });
                    i = end + 1;
                }
                // Unclosed block, or a payload this parser does not
                // recognise: skip the whole block rather than reading past it.
                None => {
                    i += 1;
                }
            }
            continue;
        }
        i += 1;
    }

    if let Some(start) = content.find("pub const LINT_METADATA: &[LintMetadata] = &[") {
        let after = &content[start + "pub const LINT_METADATA: &[LintMetadata] = &[".len()..];
        if let Some(end) = after.find("];") {
            let meta_block = &after[..end];
            let meta_entries: Vec<&str> = meta_block.split("LintMetadata {").skip(1).collect();
            for entry_str in meta_entries {
                let mut lint_var = String::new();
                let mut category = String::new();
                for line in entry_str.lines() {
                    let t = line.trim().trim_end_matches(',');
                    if let Some(val) = t.strip_prefix("lint:") {
                        lint_var = val.trim().to_string();
                    } else if let Some(val) = t.strip_prefix("category:") {
                        let raw = val.trim();
                        category = raw
                            .strip_prefix("LintCategory::")
                            .unwrap_or(raw)
                            .to_string();
                    }
                }
                if !lint_var.is_empty() && !category.is_empty() {
                    for entry in &mut entries {
                        if entry.name_snake == lint_var {
                            entry.category = Some(category.clone());
                            break;
                        }
                    }
                }
            }
        }
    }

    entries
}

/// Index of the line holding the `}` that balances the `{` opening the
/// `declare_lint!` block at `start_line`; `None` if the block never closes.
fn block_end_line(lines: &[&str], start_line: usize) -> Option<usize> {
    let text = lines[start_line..].join("\n");
    let first_line = text.find('\n').unwrap_or(text.len());
    // The opening line ends with `{`, so it is the last one on that line.
    let open = text[..first_line].rfind('{')?;
    let close = matching_close_brace(&text, open)?;
    Some(start_line + text[..close].matches('\n').count())
}

/// Reads `pub NAME,` / `LEVEL,` / `"description"` out of the lines between the
/// braces of one `declare_lint!` block. Returns `None` when the payload is not
/// shaped like that, so callers skip the block instead of guessing.
fn parse_block_body(body: &[&str]) -> Option<(String, String, String)> {
    let mut i = 0;
    while i < body.len() {
        let line = body[i].trim();
        if line.is_empty()
            || line.starts_with("///")
            || line.starts_with("//")
            || line.starts_with("#[")
        {
            i += 1;
        } else {
            break;
        }
    }

    let name_line = body.get(i)?.trim();
    if !name_line.starts_with("pub ") {
        return None;
    }
    let name_snake = name_line
        .strip_prefix("pub ")
        .unwrap_or(name_line)
        .trim_end_matches(',')
        .trim()
        .to_string();

    i += 1;
    while i < body.len() && body[i].trim().is_empty() {
        i += 1;
    }
    let level = body.get(i)?.trim().trim_end_matches(',').to_string();

    i += 1;
    while i < body.len() && body[i].trim().is_empty() {
        i += 1;
    }
    let desc = body
        .get(i)?
        .trim()
        .trim_start_matches('"')
        .trim_end_matches(',')
        .trim_end_matches('"')
        .to_string();

    Some((name_snake, level, desc))
}

/// Index of the `}` closing the `{` at `open`, so a `}` inside a string
/// literal or a comment never ends the scan early — only the brace that
/// balances `open` does.
fn matching_close_brace(content: &str, open: usize) -> Option<usize> {
    if content.as_bytes().get(open) != Some(&b'{') {
        return None;
    }

    let mut depth: u32 = 0;
    let mut i = open;
    while i < content.len() {
        let ch = content[i..].chars().next()?;
        let width = ch.len_utf8();
        let rest = &content[i + width..];

        // Comments carry no structure.
        if ch == '/' && rest.starts_with('/') {
            i = match rest.find('\n') {
                Some(newline) => i + width + newline,
                None => content.len(),
            };
            continue;
        }
        if ch == '/' && rest.starts_with('*') {
            let mut nested: u32 = 1;
            i += width + 1; // past the opening `/*`
            while nested > 0 {
                let next = content[i..].chars().next()?;
                let next_width = next.len_utf8();
                let next_rest = &content[i + next_width..];
                if next == '*' && next_rest.starts_with('/') {
                    nested -= 1;
                    i += next_width + 1;
                } else if next == '/' && next_rest.starts_with('*') {
                    nested += 1;
                    i += next_width + 1;
                } else {
                    i += next_width;
                }
            }
            continue;
        }

        // Quoted literals hide their contents from the scan.
        if ch == '"' {
            i = skip_string_literal(content, i)?;
            continue;
        }
        if ch == '\'' {
            if let Some(end) = skip_char_literal(content, i) {
                i = end;
                continue;
            }
        }
        if ch == 'r' {
            if let Some(end) = skip_raw_string(content, i) {
                i = end;
                continue;
            }
        }

        if ch == '}' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        } else if ch == '{' {
            depth += 1;
        }
        i += width;
    }
    None
}

/// Index just past the closing quote of the `"..."` literal at `start`.
fn skip_string_literal(content: &str, start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < content.len() {
        let ch = content[i..].chars().next()?;
        let width = ch.len_utf8();
        if ch == '\\' {
            i += 1;
            if let Some(escaped) = content[i..].chars().next() {
                i += escaped.len_utf8();
            }
        } else if ch == '"' {
            return Some(i + width);
        } else {
            i += width;
        }
    }
    None
}

/// If `content[start..]` opens a char literal, the index just past it; `None`
/// for a lifetime such as `'a`.
fn skip_char_literal(content: &str, start: usize) -> Option<usize> {
    let mut i = start + 1;
    if content.get(i..)?.starts_with('\\') {
        i += 1;
        i += content.get(i..)?.chars().next()?.len_utf8();
        while i < content.len() {
            let ch = content.get(i..)?.chars().next()?;
            i += ch.len_utf8();
            if ch == '\'' {
                return Some(i);
            }
            if ch == '\n' {
                return None;
            }
        }
        return None;
    }
    let first = content.get(i..)?.chars().next()?;
    i += first.len_utf8();
    if content.get(i..)?.starts_with('\'') {
        Some(i + 1)
    } else {
        None
    }
}

/// If `content[start..]` opens a raw string literal, the index just past its
/// closing delimiter; `None` otherwise.
fn skip_raw_string(content: &str, start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut hashes = 0usize;
    while content.get(i..)?.starts_with('#') {
        i += 1;
        hashes += 1;
    }
    if !content.get(i..)?.starts_with('"') {
        return None;
    }
    i += 1;
    if hashes == 0 {
        return content[i..].find('"').map(|rel| i + rel + 1);
    }
    let terminator = format!("\"{}", "#".repeat(hashes));
    content[i..]
        .find(&terminator)
        .map(|rel| i + rel + terminator.len())
}

fn generate_readme(entries: &[LintEntry]) -> String {
    let category_order = [
        "StorageOperations",
        "Compute",
        "Memory",
        "EntryLifecycle",
        "SymbolOperations",
    ];
    let category_labels: BTreeMap<&str, &str> = [
        ("StorageOperations", "Storage Operations"),
        ("Compute", "CPU/Compute"),
        ("Memory", "Memory"),
        ("EntryLifecycle", "Entry Lifecycle"),
        ("SymbolOperations", "Symbol Operations"),
    ]
    .into_iter()
    .collect();

    let mut categorized: BTreeMap<&str, Vec<&LintEntry>> = BTreeMap::new();
    for entry in entries {
        let cat = entry.category.as_deref().unwrap_or("Other");
        categorized.entry(cat).or_default().push(entry);
    }

    let mut md = String::new();
    md.push_str("# Lint Reference\n\n");
    md.push_str("<!-- DO NOT EDIT THIS FILE BY HAND -->\n");
    md.push_str("<!-- This file is generated by `tools/generate-lint-docs`. -->\n");
    md.push_str(
        "<!-- Run `cargo run -p generate-lint-docs` from the workspace root to regenerate. -->\n\n",
    );
    md.push_str("{% hint style=\"info\" %}\n");
    md.push_str("See the [Cost Rationale](../cost_rationale.md) page for a full explanation of Soroban's metered resources and why each resource matters.\n");
    md.push_str("{% endhint %}\n\n");

    for cat in &category_order {
        if let Some(entries) = categorized.get(*cat) {
            let label = category_labels.get(cat).unwrap_or(cat);
            md.push_str(&format!("## {}\n\n", label));
            md.push_str("| Lint | Default Severity | Catches |\n");
            md.push_str("| --- | --- | --- |\n");
            for entry in entries {
                let doc_path = format!("{}.md", entry.name_doc);
                md.push_str(&format!(
                    "| [`{}`]({}) | `{}` | {} |\n",
                    entry.name_doc,
                    doc_path,
                    entry.level.to_lowercase(),
                    entry.description
                ));
            }
            md.push('\n');
        }
    }

    if let Some(entries) = categorized.get("Other") {
        md.push_str("## Other\n\n");
        md.push_str("| Lint | Default Severity | Catches |\n");
        md.push_str("| --- | --- | --- |\n");
        for entry in entries {
            let doc_path = format!("{}.md", entry.name_doc);
            md.push_str(&format!(
                "| [`{}`]({}) | `{}` | {} |\n",
                entry.name_doc,
                doc_path,
                entry.level.to_lowercase(),
                entry.description
            ));
        }
        md.push('\n');
    }

    md.push_str("{% hint style=\"info\" %}\n");
    md.push_str("Severities can be adjusted per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).\n");
    md.push_str("{% endhint %}\n");

    md
}

fn generate_catalog(entries: &[LintEntry]) -> String {
    let mut md = String::new();
    md.push_str("# Lint Catalog\n\n");
    md.push_str("<!-- DO NOT EDIT THIS FILE BY HAND -->\n");
    md.push_str("<!-- This file is generated by `tools/generate-lint-docs`. -->\n");
    md.push_str(
        "<!-- Run `cargo run -p generate-lint-docs` from the workspace root to regenerate. -->\n\n",
    );
    md.push_str(
        "This document provides a concise reference for all lints supported by **soroban-cost-linter**. Each entry includes the lint name, its default severity, a brief description, and a link to the full documentation.\n\n",
    );
    md.push_str("| Lint | Default Severity | Description | Docs |\n");
    md.push_str("|------|------------------|-------------|------|\n");

    for entry in entries {
        let doc_path = format!("lints/{}.md", entry.name_doc);
        md.push_str(&format!(
            "| `{}` | {} | {} | [Link]({}) |\n",
            entry.name_doc,
            entry.level.to_lowercase(),
            entry.description,
            doc_path
        ));
    }

    md.push_str("\n*Severities can be overridden via `budget.toml`.*\n");
    md
}

fn resolve_path(path: &str) -> PathBuf {
    let explicit = std::env::args()
        .position(|a| a == "--workspace-root")
        .and_then(|idx| std::env::args().nth(idx + 1));

    if let Some(root) = explicit {
        Path::new(&root).join(path)
    } else {
        Path::new(path).to_path_buf()
    }
}

fn main() {
    let check_mode = std::env::args().any(|a| a == "--check");

    let lib_path = resolve_path("soroban_cost_lints/src/lib.rs");
    let readme_path = resolve_path("docs/lints/README.md");
    let catalog_path = resolve_path("docs/lint_catalog.md");

    if !lib_path.exists() {
        eprintln!(
            "Error: Cannot find soroban_cost_lints/src/lib.rs at {:?}",
            lib_path
        );
        eprintln!("Run from workspace root or pass --workspace-root <path>.");
        process::exit(1);
    }

    let content = fs::read_to_string(&lib_path).expect("Failed to read lib.rs");
    let entries = parse_lib_rs(&content);

    let readme = generate_readme(&entries);
    let catalog = generate_catalog(&entries);

    if check_mode {
        // Normalise line endings before comparing. On Windows, git checks these
        // files out with CRLF while the generator emits LF, so a byte-for-byte
        // comparison reports every generated file as stale on that host.
        let normalise = |s: String| s.replace("\r\n", "\n");
        let current_readme = normalise(fs::read_to_string(&readme_path).unwrap_or_default());
        let current_catalog = normalise(fs::read_to_string(&catalog_path).unwrap_or_default());

        let mut exit_code = 0;
        if current_readme != readme {
            eprintln!("❌ docs/lints/README.md is out of date. Run `cargo run -p generate-lint-docs` from workspace root to regenerate.");
            exit_code = 1;
        }
        if current_catalog != catalog {
            eprintln!("❌ docs/lint_catalog.md is out of date. Run `cargo run -p generate-lint-docs` from workspace root to regenerate.");
            exit_code = 1;
        }
        if exit_code != 0 {
            process::exit(exit_code);
        }
        println!("✅ Lint documentation is up to date.");
    } else {
        if let Some(parent) = readme_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&readme_path, &readme).expect("Failed to write README.md");
        println!("✅ Wrote {:?}", readme_path);
        if let Some(parent) = catalog_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&catalog_path, &catalog).expect("Failed to write docs/lint_catalog.md");
        println!("✅ Wrote {:?}", catalog_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    // ------------------------------------------------------------------
    // Fixture strings — kept deliberately simple so tests survive lint
    // additions. Tests use fixture strings, not the real lib.rs.
    // ------------------------------------------------------------------

    const SINGLE_LINT: &str = r#"
        rustc_session::declare_lint! {
            pub SOROBAN_STORAGE_IN_LOOP,
            Deny,
            "storage operations inside a loop"
        }
    "#;

    const TWO_LINTS: &str = r#"
        rustc_session::declare_lint! {
            pub SOROBAN_STORAGE_IN_LOOP,
            Deny,
            "storage operations inside a loop"
        }

        rustc_session::declare_lint! {
            pub REDUNDANT_ENV_CLONE,
            Warn,
            "redundant clone on Env object"
        }
    "#;

    const LINT_WITH_MULTILINE_DESC: &str = r#"
        rustc_session::declare_lint! {
            pub MY_LINT,
            Warn,
            "a lint with a
multiline description"
        }
    "#;

    const LINT_WITH_ATTRIBUTE: &str = r#"
        /// This is a doc comment
        #[allow(unused)]
        rustc_session::declare_lint! {
            pub MY_LINT,
            Warn,
            "some description"
        }
    "#;

    const LINT_UNUSUAL_WHITESPACE: &str = r#"
        rustc_session::declare_lint! {



            pub MY_LINT,



            Warn,



            "some description"
        }
    "#;

    const LINT_DESC_WITH_QUOTES: &str = r#"
        rustc_session::declare_lint! {
            pub MY_LINT,
            Warn,
            "a lint that mentions `backticks` and \"quotes\" in its description"
        }
    "#;

    const EMPTY_INPUT: &str = "";

    const NO_DECLARE_LINT: &str = r#"
        // Just some regular code
        fn main() {}
    "#;

    const MALFORMED_NO_CLOSE: &str = r#"
        rustc_session::declare_lint! {
            not_a_pub_line,
            Warn,
            "description"
        }
    "#;

    const NON_PUB_NAME: &str = r#"
        rustc_session::declare_lint! {
            not_pub,
            Warn,
            "should be skipped"
        }
    "#;

    const LINT_WITH_STRAY_BRACES: &str = r#"
        rustc_session::declare_lint! {
            // a comment holding a stray } brace
            pub STRAY_BRACE_LINT,
            Warn,
            "mentions a } brace and a /* fake */ comment"
        }
        pub const AFTER_BLOCK: &str = "must not be swallowed";
    "#;

    const LINT_WITHOUT_DESCRIPTION: &str = r#"
        rustc_session::declare_lint! {
            pub NO_DESC_LINT,
            Warn,
        }
        pub const AFTER_BLOCK: &str = "must not be swallowed";
    "#;

    const LINT_UNCLOSED: &str = r#"
        rustc_session::declare_lint! {
            pub UNCLOSED_LINT,
            Warn,
            "the block below never closes"
        pub const AFTER_BLOCK: &str = "must not be swallowed";
    "#;

    // ------------------------------------------------------------------
    // parse_lib_rs tests
    // ------------------------------------------------------------------

    #[test]
    fn parse_single_lint_extracts_name_level_description() {
        let entries = parse_lib_rs(SINGLE_LINT);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name_snake, "SOROBAN_STORAGE_IN_LOOP");
        assert_eq!(entries[0].name_doc, "soroban_storage_in_loop");
        assert_eq!(entries[0].level, "Deny");
        assert_eq!(entries[0].description, "storage operations inside a loop");
        assert!(entries[0].category.is_none());
    }

    #[test]
    fn parse_two_lints() {
        let entries = parse_lib_rs(TWO_LINTS);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name_snake, "SOROBAN_STORAGE_IN_LOOP");
        assert_eq!(entries[1].name_snake, "REDUNDANT_ENV_CLONE");
        assert_eq!(entries[1].level, "Warn");
    }

    #[test]
    fn parse_deny_level() {
        let entries = parse_lib_rs(SINGLE_LINT);
        assert_eq!(entries[0].level, "Deny");
    }

    #[test]
    fn parse_warn_level() {
        let entries = parse_lib_rs(TWO_LINTS);
        assert_eq!(entries[1].level, "Warn");
    }

    #[test]
    fn parse_multiline_description() {
        let entries = parse_lib_rs(LINT_WITH_MULTILINE_DESC);
        assert_eq!(entries.len(), 1);
        // The parser reads one line for the description; the raw string has a literal
        // newline inside the quotes, so the parser captures only the first line.
        assert_eq!(entries[0].description, "a lint with a");
    }

    #[test]
    fn parse_description_with_quotes_and_backticks() {
        let entries = parse_lib_rs(LINT_DESC_WITH_QUOTES);
        assert_eq!(entries.len(), 1);
        // The description should contain the literal backtick and quote characters
        assert!(entries[0].description.contains("backticks"));
        assert!(entries[0].description.contains("quotes"));
    }

    #[test]
    fn parse_attribute_between_doc_comment_and_macro() {
        let entries = parse_lib_rs(LINT_WITH_ATTRIBUTE);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name_snake, "MY_LINT");
        assert_eq!(entries[0].description, "some description");
    }

    #[test]
    fn parse_unusual_whitespace() {
        // Extra blank lines between fields — rustfmt may produce these.
        let entries = parse_lib_rs(LINT_UNUSUAL_WHITESPACE);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name_snake, "MY_LINT");
        assert_eq!(entries[0].level, "Warn");
        assert_eq!(entries[0].description, "some description");
    }

    #[test]
    fn parse_empty_input_returns_empty() {
        let entries = parse_lib_rs(EMPTY_INPUT);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_no_declare_lint_returns_empty() {
        let entries = parse_lib_rs(NO_DECLARE_LINT);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_non_pub_name_is_skipped() {
        let entries = parse_lib_rs(NON_PUB_NAME);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_malformed_input_does_not_panic() {
        // A non-pub name should not panic — it just produces no entries.
        let result = std::panic::catch_unwind(|| parse_lib_rs(MALFORMED_NO_CLOSE));
        assert!(result.is_ok(), "parse_lib_rs panicked on malformed input");
        let entries = result.unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn parse_with_lint_metadata_block() {
        let input = format!(
            "{}\n\n{}",
            TWO_LINTS,
            r#"
                pub const LINT_METADATA: &[LintMetadata] = &[
                    LintMetadata {
                        lint: SOROBAN_STORAGE_IN_LOOP,
                        category: LintCategory::StorageOperations,
                    },
                    LintMetadata {
                        lint: REDUNDANT_ENV_CLONE,
                        category: LintCategory::Memory,
                    },
                ];
            "#
        );
        let entries = parse_lib_rs(&input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].category.as_deref(), Some("StorageOperations"));
        assert_eq!(entries[1].category.as_deref(), Some("Memory"));
    }

    #[test]
    fn parse_block_ends_at_matching_brace() {
        let entries = parse_lib_rs(LINT_WITH_STRAY_BRACES);
        assert_eq!(entries.len(), 1, "entries: {:?}", entries);
        assert_eq!(entries[0].name_snake, "STRAY_BRACE_LINT");
        assert_eq!(entries[0].level, "Warn");
        assert_eq!(
            entries[0].description,
            "mentions a } brace and a /* fake */ comment"
        );
    }

    #[test]
    fn parse_does_not_read_past_closing_brace() {
        // The `}` line is not a description; anything after the block belongs
        // to whatever comes next.
        let entries = parse_lib_rs(LINT_WITHOUT_DESCRIPTION);
        assert!(entries.is_empty(), "entries: {:?}", entries);
    }

    #[test]
    fn parse_unclosed_block_is_skipped() {
        let entries = parse_lib_rs(LINT_UNCLOSED);
        assert!(entries.is_empty(), "entries: {:?}", entries);
    }

    // ------------------------------------------------------------------
    // generate_readme tests
    // ------------------------------------------------------------------

    #[test]
    fn generate_readme_empty_entries() {
        let md = generate_readme(&[]);
        assert!(md.starts_with("# Lint Reference"));
        assert!(md.contains("DO NOT EDIT THIS FILE BY HAND"));
        assert!(md.contains("{% hint style=\"info\" %}"));
    }

    #[test]
    fn generate_readme_single_lint_in_other_category() {
        let entries = parse_lib_rs(SINGLE_LINT);
        // SOROBAN_STORAGE_IN_LOOP has no metadata block, so category is None -> "Other"
        let md = generate_readme(&entries);
        assert!(md.contains("## Other"));
        assert!(md.contains("`soroban_storage_in_loop`"));
        assert!(md.contains("`deny`"));
        assert!(md.contains("storage operations inside a loop"));
    }

    #[test]
    fn generate_readme_categorized_lint() {
        let input = format!(
            "{}\n\n{}",
            SINGLE_LINT,
            r#"
                pub const LINT_METADATA: &[LintMetadata] = &[
                    LintMetadata {
                        lint: SOROBAN_STORAGE_IN_LOOP,
                        category: LintCategory::StorageOperations,
                    },
                ];
            "#
        );
        let entries = parse_lib_rs(&input);
        let md = generate_readme(&entries);
        assert!(md.contains("## Storage Operations"));
        assert!(!md.contains("## Other"));
    }

    #[test]
    fn generate_readme_multiple_categories_ordered() {
        let input = format!(
            "{}\n\n{}",
            TWO_LINTS,
            r#"
                pub const LINT_METADATA: &[LintMetadata] = &[
                    LintMetadata {
                        lint: SOROBAN_STORAGE_IN_LOOP,
                        category: LintCategory::StorageOperations,
                    },
                    LintMetadata {
                        lint: REDUNDANT_ENV_CLONE,
                        category: LintCategory::Memory,
                    },
                ];
            "#
        );
        let entries = parse_lib_rs(&input);
        let md = generate_readme(&entries);
        // StorageOperations should appear before Memory
        let storage_pos = md.find("## Storage Operations").unwrap();
        let memory_pos = md.find("## Memory").unwrap();
        assert!(storage_pos < memory_pos);
    }

    // ------------------------------------------------------------------
    // Golden-file tests — compare generated output against the real files
    // ------------------------------------------------------------------

    #[test]
    fn golden_readme_matches_actual_file() {
        let ws = workspace_root();
        let content = fs::read_to_string(ws.join("soroban_cost_lints/src/lib.rs"))
            .expect("Failed to read lib.rs — run from workspace root");
        let entries = parse_lib_rs(&content);
        let generated = generate_readme(&entries);
        let actual = fs::read_to_string(ws.join("docs/lints/README.md"))
            .expect("Failed to read docs/lints/README.md");
        let actual = actual.replace("\r\n", "\n");
        assert_eq!(
            generated, actual,
            "docs/lints/README.md is stale — regenerate with `cargo run -p generate-lint-docs`"
        );
    }

    #[test]
    fn golden_catalog_matches_actual_file() {
        let ws = workspace_root();
        let content = fs::read_to_string(ws.join("soroban_cost_lints/src/lib.rs"))
            .expect("Failed to read lib.rs — run from workspace root");
        let entries = parse_lib_rs(&content);
        let generated = generate_catalog(&entries);
        let actual = fs::read_to_string(ws.join("docs/lint_catalog.md"))
            .expect("Failed to read docs/lint_catalog.md");
        let actual = actual.replace("\r\n", "\n");
        assert_eq!(
            generated, actual,
            "docs/lint_catalog.md is stale — regenerate with `cargo run -p generate-lint-docs`"
        );
    }

    // ------------------------------------------------------------------
    // --check mode integration tests (subprocess)
    // ------------------------------------------------------------------

    fn workspace_root() -> PathBuf {
        // When tests run, cwd is the crate directory (tools/generate-lint-docs).
        // The workspace root is two levels up.
        PathBuf::from("../..")
            .canonicalize()
            .expect("Failed to resolve workspace root")
    }

    fn find_binary() -> PathBuf {
        static INIT: std::sync::Once = std::sync::Once::new();
        let ws = workspace_root();
        let path = ws.join("target/debug/generate-lint-docs");
        INIT.call_once(|| {
            if !path.exists() {
                let status = Command::new("cargo")
                    .args(["build", "-p", "generate-lint-docs"])
                    .current_dir(&ws)
                    .status()
                    .expect("Failed to run cargo build");
                assert!(status.success(), "cargo build -p generate-lint-docs failed");
            }
        });
        path
    }

    #[test]
    fn check_mode_passes_when_files_match() {
        let bin = find_binary();
        let ws = workspace_root();
        let status = Command::new(&bin)
            .args(["--check", "--workspace-root", ws.to_str().unwrap()])
            .status()
            .expect("Failed to execute binary");
        assert!(
            status.success(),
            "--check should pass when generated files are up to date"
        );
    }

    #[test]
    fn check_mode_fails_when_readme_is_stale() {
        let bin = find_binary();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Replicate the workspace layout the binary expects.
        let fake_lib = root.join("soroban_cost_lints/src/lib.rs");
        fs::create_dir_all(fake_lib.parent().unwrap()).unwrap();
        fs::write(&fake_lib, SINGLE_LINT).unwrap();

        let readme_path = root.join("docs/lints/README.md");
        fs::create_dir_all(readme_path.parent().unwrap()).unwrap();
        fs::write(&readme_path, "STALE CONTENT").unwrap();

        let entries = parse_lib_rs(SINGLE_LINT);
        let correct_catalog = generate_catalog(&entries);
        let catalog_path = root.join("docs/lint_catalog.md");
        fs::write(&catalog_path, &correct_catalog).unwrap();

        let catalog_path = root.join("docs/lint_catalog.md");
        fs::create_dir_all(catalog_path.parent().unwrap()).unwrap();
        let correct_catalog = generate_catalog(&entries);
        fs::write(&catalog_path, &correct_catalog).unwrap();

        let status = Command::new(&bin)
            .args(["--check", "--workspace-root", root.to_str().unwrap()])
            .status()
            .expect("Failed to execute binary");
        assert!(
            !status.success(),
            "--check should fail when README is stale"
        );
    }

    #[test]
    fn check_mode_fails_when_catalog_is_stale() {
        let bin = find_binary();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let fake_lib = root.join("soroban_cost_lints/src/lib.rs");
        fs::create_dir_all(fake_lib.parent().unwrap()).unwrap();
        fs::write(&fake_lib, SINGLE_LINT).unwrap();

        let entries = parse_lib_rs(SINGLE_LINT);
        let readme_path = root.join("docs/lints/README.md");
        fs::create_dir_all(readme_path.parent().unwrap()).unwrap();
        fs::write(&readme_path, generate_readme(&entries)).unwrap();

        let catalog_path = root.join("docs/lint_catalog.md");
        fs::write(&catalog_path, "STALE CONTENT").unwrap();

        let status = Command::new(&bin)
            .args(["--check", "--workspace-root", root.to_str().unwrap()])
            .status()
            .expect("Failed to execute binary");
        assert!(
            !status.success(),
            "--check should fail when catalog is stale"
        );
    }

    #[test]
    fn check_mode_passes_when_all_files_match() {
        let bin = find_binary();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let fake_lib = root.join("soroban_cost_lints/src/lib.rs");
        fs::create_dir_all(fake_lib.parent().unwrap()).unwrap();
        fs::write(&fake_lib, SINGLE_LINT).unwrap();

        let entries = parse_lib_rs(SINGLE_LINT);

        let readme_path = root.join("docs/lints/README.md");
        fs::create_dir_all(readme_path.parent().unwrap()).unwrap();
        fs::write(&readme_path, generate_readme(&entries)).unwrap();

        let catalog_path = root.join("docs/lint_catalog.md");
        fs::write(&catalog_path, generate_catalog(&entries)).unwrap();

        let status = Command::new(&bin)
            .args(["--check", "--workspace-root", root.to_str().unwrap()])
            .status()
            .expect("Failed to execute binary");
        assert!(
            status.success(),
            "--check should pass when all generated files match"
        );
    }
}

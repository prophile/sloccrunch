use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Parser;
use file_type::FileType;
use ignore::WalkBuilder;
use linguist::DetectedLanguage;
use linguist_types::LanguageType;

#[derive(Parser, Debug)]
#[command(name = "sloccrunch", about = "Count source lines of code in a directory tree")]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ProjectTotals {
    total_sloc: usize,
    by_language: HashMap<String, usize>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let totals = count_project(&cli.path)?;

    let mut by_language: Vec<_> = totals.by_language.into_iter().collect();
    by_language.sort_by(|(left_name, left_total), (right_name, right_total)| {
        right_total
            .cmp(left_total)
            .then_with(|| left_name.cmp(right_name))
    });

    for (language, total) in by_language {
        println!("{language}: {total}");
    }

    println!("Total SLOC: {}", totals.total_sloc);

    Ok(())
}

fn count_project(root: &Path) -> io::Result<ProjectTotals> {
    let mut totals = ProjectTotals::default();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .parents(true)
        .require_git(false)
        .build();

    for entry in walker {
        let entry = entry.map_err(io::Error::other)?;
        match entry.file_type() {
            Some(file_type) if file_type.is_file() => {}
            _ => continue,
        }

        let path = entry.path();
        let Some(language) = detect_language(path)? else {
            continue;
        };

        let sloc = count_physical_lines(path)?;
        totals.total_sloc += sloc;
        *totals.by_language.entry(language.to_owned()).or_default() += sloc;
    }

    Ok(totals)
}

fn detect_language(path: &Path) -> io::Result<Option<&'static str>> {
    let file_type = FileType::try_from_file(path)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    if file_type.name() == "Binary" {
        return Ok(None);
    }

    let file_contents = fs::read(path)?;
    let file_contents = String::from_utf8_lossy(&file_contents);
    let all_candidates = collect_language_candidates(path)?;
    if is_known_non_source_extension(path, &all_candidates) {
        return Ok(None);
    }
    if let Some(language) = guess_programming_language(path, &file_contents, &all_candidates) {
        return Ok(Some(language));
    }

    if all_candidates.is_empty() {
        return Ok(None);
    }

    if all_candidates.len() == 1 {
        return Ok(all_candidates
            .into_iter()
            .next()
            .filter(|language| language.definition.language_type == LanguageType::Programming)
            .map(|language| language.definition.group.as_deref().unwrap_or(language.name)));
    }

    let mut programming_candidates = filter_programming_languages(all_candidates.clone());

    if programming_candidates.len() == 1 {
        return Ok(programming_candidates.into_iter().next().map(|language| {
            language
                .definition
                .group
                .as_deref()
                .unwrap_or(language.name)
        }));
    }

    if programming_candidates.len() > 1 {
        let disambiguated = linguist::disambiguate(path, &file_contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        let disambiguated = filter_programming_languages(disambiguated);
        if !disambiguated.is_empty() {
            programming_candidates = disambiguated;
        }
    }

    if programming_candidates.is_empty() || programming_candidates.len() != all_candidates.len() {
        return Ok(None);
    }

    Ok(programming_candidates.into_iter().next().map(|language| {
        language
            .definition
            .group
            .as_deref()
            .unwrap_or(language.name)
    }))
}

fn guess_programming_language(
    path: &Path,
    file_contents: &str,
    candidates: &[DetectedLanguage],
) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();

    match extension.as_str() {
        "rs" if candidates.iter().any(|candidate| candidate.name == "Rust") => {
            if looks_like_rust(file_contents) {
                return Some("Rust");
            }
        }
        _ => {}
    }

    None
}

fn is_known_non_source_extension(path: &Path, candidates: &[DetectedLanguage]) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "md" | "markdown" | "mdown" | "mdwn" | "mkd" | "mkdn" | "mkdown" | "livemd"
    ) && candidates.iter().any(|candidate| candidate.name == "Markdown")
}

fn collect_language_candidates(path: &Path) -> io::Result<Vec<DetectedLanguage>> {
    let mut candidates = normalize_candidates(
        linguist::detect_language_by_filename(path)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?,
    );
    let from_extension = normalize_candidates(
        linguist::detect_language_by_extension(path)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?,
    );

    for language in from_extension {
        if candidates.iter().any(|candidate| candidate.name == language.name) {
            continue;
        }
        candidates.push(language);
    }

    Ok(candidates)
}

fn normalize_candidates(candidates: Vec<DetectedLanguage>) -> Vec<DetectedLanguage> {
    let mut normalized = Vec::new();

    for language in candidates {
        let name = language.definition.group.as_deref().unwrap_or(language.name);
        if normalized.iter().any(|candidate: &DetectedLanguage| {
            candidate.definition.group.as_deref().unwrap_or(candidate.name) == name
        }) {
            continue;
        }
        normalized.push(language);
    }

    normalized
}

fn filter_programming_languages(candidates: Vec<DetectedLanguage>) -> Vec<DetectedLanguage> {
    candidates
        .into_iter()
        .filter(|candidate| candidate.definition.language_type == LanguageType::Programming)
        .collect()
}

fn looks_like_rust(file_contents: &str) -> bool {
    let rust_markers = [
        "fn ",
        "let ",
        "use ",
        "pub ",
        "mod ",
        "impl ",
        "trait ",
        "enum ",
        "struct ",
        "::",
        "println!",
        "format!",
        "match ",
    ];

    rust_markers
        .iter()
        .any(|marker| file_contents.contains(marker))
}

fn count_physical_lines(path: &Path) -> io::Result<usize> {
    let bytes = fs::read(path)?;
    Ok(count_lines(&bytes))
}

fn count_lines(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }

    let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
    newline_count + usize::from(bytes.last() != Some(&b'\n'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn counts_empty_input() {
        assert_eq!(count_lines(b""), 0);
    }

    #[test]
    fn counts_trailing_newline_lines() {
        assert_eq!(count_lines(b"one\ntwo\n"), 2);
    }

    #[test]
    fn counts_missing_trailing_newline() {
        assert_eq!(count_lines(b"one\ntwo"), 2);
    }

    #[test]
    fn counts_project_totals_and_gitignores() -> io::Result<()> {
        let fixture = TempProject::new()?;
        fixture.write(".gitignore", "ignored.rs\nnested/\n")?;
        fixture.write("src/main.rs", "fn main() {\n    println!(\"hi\");\n}\n")?;
        fixture.write("src/lib.rs", "pub fn answer() -> u32 {\n    42\n}\n")?;
        fixture.write("ignored.rs", "fn skipped() {}\n")?;
        fixture.write("README.md", "# title\nbody\n")?;
        fixture.write("nested/skip.rs", "fn also_skipped() {}\n")?;

        let totals = count_project(fixture.path())?;

        assert_eq!(totals.total_sloc, 6);
        assert_eq!(totals.by_language.get("Rust"), Some(&6));
        assert_eq!(totals.by_language.len(), 1);

        Ok(())
    }

    #[test]
    fn counts_typescript_when_extension_has_non_programming_ambiguity() -> io::Result<()> {
        let fixture = TempProject::new()?;
        fixture.write(
            "src/index.ts",
            "export const answer: number = 42;\nfunction greet(name: string): string {\n  return name;\n}\n",
        )?;

        let totals = count_project(fixture.path())?;

        assert_eq!(totals.total_sloc, 4);
        assert_eq!(totals.by_language.get("TypeScript"), Some(&4));

        Ok(())
    }

    #[test]
    fn includes_hidden_files_unless_gitignored() -> io::Result<()> {
        let fixture = TempProject::new()?;
        fixture.write(".hidden.rs", "fn visible() {}\n")?;

        let totals = count_project(fixture.path())?;

        assert_eq!(totals.total_sloc, 1);
        assert_eq!(totals.by_language.get("Rust"), Some(&1));

        Ok(())
    }

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new() -> io::Result<Self> {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_nanos();
            let root = std::env::temp_dir().join(format!("sloccrunch-{unique}"));
            fs::create_dir(&root)?;
            Ok(Self { root })
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn write(&self, relative_path: &str, contents: &str) -> io::Result<()> {
            let path = self.root.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, contents)
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            if self.root.file_name() != Some(OsStr::new("")) {
                let _ = fs::remove_dir_all(&self.root);
            }
        }
    }
}

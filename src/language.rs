use std::fs;
use std::io;
use std::path::Path;

use file_type::FileType;
use linguist::DetectedLanguage;
use linguist_types::LanguageType;

pub(crate) fn detect_language(path: &Path) -> io::Result<Option<&'static str>> {
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

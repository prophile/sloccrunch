use std::collections::HashMap;
use std::io;
use std::path::Path;

use ignore::WalkBuilder;

use crate::language::detect_language;
use crate::lines::count_physical_lines;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ProjectTotals {
    pub(crate) total_sloc: usize,
    pub(crate) by_language: HashMap<String, usize>,
}

pub(crate) fn count_project(root: &Path) -> io::Result<ProjectTotals> {
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

#[cfg(test)]
mod tests {
    use super::count_project;
    use std::ffi::OsStr;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

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

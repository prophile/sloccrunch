use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use ignore::WalkBuilder;

use crate::language::detect_language;
use crate::lines::count_physical_lines;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ProjectTotals {
    pub(crate) total_sloc: usize,
    pub(crate) by_language: HashMap<String, usize>,
}

struct FileStats {
    language: &'static str,
    sloc: usize,
}

pub(crate) fn count_project(root: &Path, threads: usize) -> io::Result<ProjectTotals> {
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
    let worker_count = threads.max(1);
    let (path_sender, path_receiver) = mpsc::sync_channel::<PathBuf>(worker_count * 4);
    let (result_sender, result_receiver) = mpsc::channel::<io::Result<Option<FileStats>>>();
    let shared_receiver = Arc::new(Mutex::new(path_receiver));
    let mut workers = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let path_receiver = Arc::clone(&shared_receiver);
        let result_sender = result_sender.clone();
        workers.push(thread::spawn(move || {
            loop {
                let path = match path_receiver.lock() {
                    Ok(receiver) => receiver.recv(),
                    Err(_) => return,
                };

                let path = match path {
                    Ok(path) => path,
                    Err(_) => return,
                };

                let result = process_file(&path);
                if result_sender.send(result).is_err() {
                    return;
                }
            }
        }));
    }

    drop(result_sender);

    let mut file_count = 0usize;
    let mut walk_error = None;

    for entry in walker {
        let entry = match entry.map_err(io::Error::other) {
            Ok(entry) => entry,
            Err(error) => {
                walk_error = Some(error);
                break;
            }
        };
        match entry.file_type() {
            Some(file_type) if file_type.is_file() => {}
            _ => continue,
        }

        if let Err(error) = path_sender.send(entry.path().to_path_buf()) {
            walk_error = Some(io::Error::other(error.to_string()));
            break;
        }
        file_count += 1;
    }

    drop(path_sender);

    let mut processing_error = None;

    for _ in 0..file_count {
        let result = result_receiver
            .recv()
            .map_err(|error| io::Error::other(error.to_string()))?;
        match result {
            Ok(Some(file_stats)) => {
                totals.total_sloc += file_stats.sloc;
                *totals
                    .by_language
                    .entry(file_stats.language.to_owned())
                    .or_default() += file_stats.sloc;
            }
            Ok(None) => {}
            Err(error) if processing_error.is_none() => processing_error = Some(error),
            Err(_) => {}
        }
    }

    join_workers(workers)?;

    if let Some(error) = walk_error {
        return Err(error);
    }
    if let Some(error) = processing_error {
        return Err(error);
    }

    Ok(totals)
}

fn process_file(path: &Path) -> io::Result<Option<FileStats>> {
    let Some(language) = detect_language(path)? else {
        return Ok(None);
    };

    let sloc = count_physical_lines(path)?;
    Ok(Some(FileStats { language, sloc }))
}

fn join_workers(workers: Vec<thread::JoinHandle<()>>) -> io::Result<()> {
    for worker in workers {
        worker
            .join()
            .map_err(|_| io::Error::other("worker thread panicked"))?;
    }

    Ok(())
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

        let totals = count_project(fixture.path(), 2)?;

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

        let totals = count_project(fixture.path(), 2)?;

        assert_eq!(totals.total_sloc, 4);
        assert_eq!(totals.by_language.get("TypeScript"), Some(&4));

        Ok(())
    }

    #[test]
    fn includes_hidden_files_unless_gitignored() -> io::Result<()> {
        let fixture = TempProject::new()?;
        fixture.write(".hidden.rs", "fn visible() {}\n")?;

        let totals = count_project(fixture.path(), 2)?;

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

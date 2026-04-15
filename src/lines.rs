use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn count_physical_lines(path: &Path) -> io::Result<usize> {
    let bytes = fs::read(path)?;
    Ok(count_lines(&bytes))
}

pub(crate) fn count_lines(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }

    let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
    newline_count + usize::from(bytes.last() != Some(&b'\n'))
}

#[cfg(test)]
mod tests {
    use super::count_lines;

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
}

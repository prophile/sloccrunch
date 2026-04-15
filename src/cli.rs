use std::path::PathBuf;
use std::thread;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sloccrunch", about = "Count source lines of code in a directory tree")]
pub(crate) struct Cli {
    #[arg(default_value = ".")]
    pub(crate) path: PathBuf,

    #[arg(
        short = 'j',
        long = "threads",
        default_value_t = default_thread_count(),
        value_parser = parse_thread_count
    )]
    pub(crate) threads: usize,
}

pub(crate) fn default_thread_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get().saturating_sub(1).max(1))
        .unwrap_or(1)
}

fn parse_thread_count(value: &str) -> Result<usize, String> {
    let threads = value
        .parse::<usize>()
        .map_err(|_| format!("invalid thread count: {value}"))?;
    if threads == 0 {
        return Err("thread count must be at least 1".to_owned());
    }
    Ok(threads)
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, default_thread_count};

    #[test]
    fn parses_threads_flag() {
        let cli = Cli::parse_from(["sloccrunch", "--threads", "4"]);
        assert_eq!(cli.threads, 4);
    }

    #[test]
    fn default_thread_count_is_at_least_one() {
        assert!(default_thread_count() >= 1);
    }

    #[test]
    fn rejects_zero_threads() {
        assert!(Cli::try_parse_from(["sloccrunch", "--threads", "0"]).is_err());
    }
}

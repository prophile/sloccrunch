use std::path::PathBuf;
use std::thread;

use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(
    name = "sloccrunch",
    about = "Count source lines of code in a directory tree"
)]
pub(crate) struct Cli {
    #[arg(default_value = ".", num_args = 1..)]
    pub(crate) paths: Vec<PathBuf>,

    #[arg(
        short = 'j',
        long = "threads",
        default_value_t = default_thread_count(),
        value_parser = parse_thread_count
    )]
    pub(crate) threads: usize,

    #[arg(long = "costs", action = ArgAction::SetTrue, overrides_with = "no_costs")]
    pub(crate) costs: bool,

    #[arg(
        long = "no-costs",
        action = ArgAction::SetTrue,
        overrides_with = "costs"
    )]
    no_costs: bool,

    #[arg(long = "salary", default_value_t = 60_000.0, value_parser = parse_salary)]
    pub(crate) salary: f64,
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

fn parse_salary(value: &str) -> Result<f64, String> {
    let salary = value
        .parse::<f64>()
        .map_err(|_| format!("invalid salary: {value}"))?;
    if salary <= 0.0 {
        return Err("salary must be greater than 0".to_owned());
    }
    Ok(salary)
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use std::path::PathBuf;

    use super::{Cli, default_thread_count};

    #[test]
    fn defaults_to_current_directory_when_no_paths_are_provided() {
        let cli = Cli::parse_from(["sloccrunch"]);
        assert_eq!(cli.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn parses_multiple_paths() {
        let cli = Cli::parse_from(["sloccrunch", "backend", "frontend"]);
        assert_eq!(
            cli.paths,
            vec![PathBuf::from("backend"), PathBuf::from("frontend")]
        );
    }

    #[test]
    fn parses_threads_flag() {
        let cli = Cli::parse_from(["sloccrunch", "--threads", "4"]);
        assert_eq!(cli.threads, 4);
    }

    #[test]
    fn parses_costs_flag() {
        let cli = Cli::parse_from(["sloccrunch", "--costs"]);
        assert!(cli.costs);
    }

    #[test]
    fn parses_no_costs_flag() {
        let cli = Cli::parse_from(["sloccrunch", "--no-costs"]);
        assert!(!cli.costs);
    }

    #[test]
    fn last_cost_flag_wins() {
        let cli = Cli::parse_from(["sloccrunch", "--costs", "--no-costs", "--costs"]);
        assert!(cli.costs);
    }

    #[test]
    fn parses_salary_flag() {
        let cli = Cli::parse_from(["sloccrunch", "--salary", "75000"]);
        assert_eq!(cli.salary, 75_000.0);
    }

    #[test]
    fn default_thread_count_is_at_least_one() {
        assert!(default_thread_count() >= 1);
    }

    #[test]
    fn rejects_zero_threads() {
        assert!(Cli::try_parse_from(["sloccrunch", "--threads", "0"]).is_err());
    }

    #[test]
    fn rejects_zero_salary() {
        assert!(Cli::try_parse_from(["sloccrunch", "--salary", "0"]).is_err());
    }

    #[test]
    fn rejects_negative_salary() {
        assert!(Cli::try_parse_from(["sloccrunch", "--salary", "-1"]).is_err());
    }
}

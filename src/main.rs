mod cli;
mod language;
mod lines;
mod project;

use std::io;

use clap::Parser;

use crate::cli::Cli;
use crate::project::count_project;

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

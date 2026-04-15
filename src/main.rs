mod cli;
mod cocomo;
mod language;
mod lines;
mod project;

use std::io;

use clap::Parser;

use crate::cli::Cli;
use crate::cocomo::{estimate_nominal, format_output as format_cocomo_output};
use crate::project::{ProjectTotals, count_project};

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let totals = count_project(&cli.path, cli.threads)?;
    let output = render_output(&totals, cli.costs, cli.salary);

    for line in output {
        println!("{line}");
    }

    Ok(())
}

fn render_output(totals: &ProjectTotals, show_costs: bool, salary: f64) -> Vec<String> {
    let mut by_language: Vec<_> = totals.by_language.iter().collect();
    by_language.sort_by(|(left_name, left_total), (right_name, right_total)| {
        right_total
            .cmp(left_total)
            .then_with(|| left_name.cmp(right_name))
    });

    let mut lines = Vec::with_capacity(by_language.len() + 8);
    for (language, total) in by_language {
        lines.push(format!("{language}: {total}"));
    }

    lines.push(format!("Total SLOC: {}", totals.total_sloc));

    if show_costs {
        let estimate = estimate_nominal(totals.total_sloc, salary);
        lines.extend(format_cocomo_output(&estimate));
    }

    lines
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::project::ProjectTotals;

    use super::render_output;

    #[test]
    fn render_output_without_costs_keeps_existing_shape() {
        let totals = ProjectTotals {
            total_sloc: 10,
            by_language: HashMap::from([("Rust".to_owned(), 7), ("TypeScript".to_owned(), 3)]),
        };

        let lines = render_output(&totals, false, 60_000.0);

        assert_eq!(
            lines,
            vec![
                "Rust: 7".to_owned(),
                "TypeScript: 3".to_owned(),
                "Total SLOC: 10".to_owned(),
            ]
        );
    }

    #[test]
    fn render_output_with_costs_appends_cost_section_after_total_sloc() {
        let totals = ProjectTotals {
            total_sloc: 1_000,
            by_language: HashMap::from([("Rust".to_owned(), 1_000)]),
        };

        let lines = render_output(&totals, true, 60_000.0);

        assert_eq!(lines[0], "Rust: 1000");
        assert_eq!(lines[1], "Total SLOC: 1000");
        assert_eq!(lines[2], "Nominal COCOMO II:");
        assert!(lines[3].contains("KSLOC = 1000 / 1000 = 1.000"));
        assert!(lines[4].contains("person-months"));
        assert!(lines[5].contains("months"));
        assert!(lines[6].contains("developers"));
        assert!(lines[7].contains("Salary/month"));
        assert!(lines[8].contains("Estimated cost"));
    }
}

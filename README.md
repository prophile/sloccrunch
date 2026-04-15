# sloccrunch

A modern version of the classic `sloccount` tool. Count source lines of code in a directory tree
and print the project total along with a per-language breakdown.

Use `--costs` to append a nominal COCOMO II estimate after the total SLOC line, including
intermediate calculation steps. Use `--no-costs` to force that section off, and `--salary`
to override the default annual developer salary of `60000` EUR used for the estimate.

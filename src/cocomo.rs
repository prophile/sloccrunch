const EFFORT_MULTIPLIER: f64 = 2.94;
const EFFORT_EXPONENT: f64 = 1.0997;
const SCHEDULE_MULTIPLIER: f64 = 3.67;
const SCHEDULE_EXPONENT: f64 = 0.3179;
const MONTHS_PER_YEAR: f64 = 12.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CocomoEstimate {
    pub(crate) total_sloc: usize,
    pub(crate) ksloc: f64,
    pub(crate) effort_person_months: f64,
    pub(crate) schedule_months: f64,
    pub(crate) average_staffing: f64,
    pub(crate) annual_salary: f64,
    pub(crate) monthly_salary: f64,
    pub(crate) total_cost: f64,
}

pub(crate) fn estimate_nominal(total_sloc: usize, annual_salary: f64) -> CocomoEstimate {
    let ksloc = total_sloc as f64 / 1_000.0;
    let effort_person_months = if total_sloc == 0 {
        0.0
    } else {
        EFFORT_MULTIPLIER * ksloc.powf(EFFORT_EXPONENT)
    };
    let schedule_months = if effort_person_months == 0.0 {
        0.0
    } else {
        SCHEDULE_MULTIPLIER * effort_person_months.powf(SCHEDULE_EXPONENT)
    };
    let average_staffing = if schedule_months == 0.0 {
        0.0
    } else {
        effort_person_months / schedule_months
    };
    let monthly_salary = annual_salary / MONTHS_PER_YEAR;
    let total_cost = effort_person_months * monthly_salary;

    CocomoEstimate {
        total_sloc,
        ksloc,
        effort_person_months,
        schedule_months,
        average_staffing,
        annual_salary,
        monthly_salary,
        total_cost,
    }
}

pub(crate) fn format_output(estimate: &CocomoEstimate) -> Vec<String> {
    let average_staffing_line = if estimate.schedule_months == 0.0 {
        "  Average staffing = 0.000 developers".to_owned()
    } else {
        format!(
            "  Average staffing = {:.3} / {:.3} = {:.3} developers",
            estimate.effort_person_months, estimate.schedule_months, estimate.average_staffing
        )
    };

    vec![
        "Nominal COCOMO II:".to_owned(),
        format!(
            "  KSLOC = {} / 1000 = {:.3}",
            estimate.total_sloc, estimate.ksloc
        ),
        format!(
            "  Effort = {EFFORT_MULTIPLIER:.2} x KSLOC^{EFFORT_EXPONENT:.4} = {:.3} person-months",
            estimate.effort_person_months
        ),
        format!(
            "  Schedule = {SCHEDULE_MULTIPLIER:.2} x PM^{SCHEDULE_EXPONENT:.4} = {:.3} months",
            estimate.schedule_months
        ),
        average_staffing_line,
        format!(
            "  Salary/month = EUR {:.2} / {MONTHS_PER_YEAR:.0} = EUR {:.2}",
            estimate.annual_salary, estimate.monthly_salary
        ),
        format!(
            "  Estimated cost = {:.3} x EUR {:.2} = EUR {:.2}",
            estimate.effort_person_months, estimate.monthly_salary, estimate.total_cost
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{estimate_nominal, format_output};

    #[test]
    fn estimates_nominal_cocomo_values() {
        let estimate = estimate_nominal(10_000, 60_000.0);

        assert!((estimate.ksloc - 10.0).abs() < 1e-9);
        assert!((estimate.effort_person_months - 36.986_848_670_278_09).abs() < 1e-6);
        assert!((estimate.schedule_months - 11.565_071_193_470_915).abs() < 1e-6);
        assert!((estimate.average_staffing - 3.198_151_403_612_551_6).abs() < 1e-6);
        assert!((estimate.monthly_salary - 5_000.0).abs() < 1e-9);
        assert!((estimate.total_cost - 184_934.243_351_390_47).abs() < 1e-6);
    }

    #[test]
    fn handles_zero_sloc_without_division_errors() {
        let estimate = estimate_nominal(0, 60_000.0);

        assert_eq!(estimate.ksloc, 0.0);
        assert_eq!(estimate.effort_person_months, 0.0);
        assert_eq!(estimate.schedule_months, 0.0);
        assert_eq!(estimate.average_staffing, 0.0);
        assert_eq!(estimate.total_cost, 0.0);
    }

    #[test]
    fn formats_intermediate_steps() {
        let estimate = estimate_nominal(1_000, 60_000.0);
        let lines = format_output(&estimate);

        assert_eq!(lines[0], "Nominal COCOMO II:");
        assert!(lines[1].contains("KSLOC = 1000 / 1000 = 1.000"));
        assert!(lines[2].contains("person-months"));
        assert!(lines[3].contains("months"));
        assert!(lines[4].contains("developers"));
        assert!(lines[5].contains("Salary/month"));
        assert!(lines[6].contains("Estimated cost"));
    }
}

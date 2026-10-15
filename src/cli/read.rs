//! `read`: detect, then poll the station's profile once in the same session,
//! print every field with its `verified` flag, and fail when any field is
//! missing or implausible (RULES.md v0.3 gate: ok = exit 0).

use anyhow::{anyhow, bail};
use bluetti_controller::protocol::device::{Model, Problem, Profile};
use bluetti_controller::transport::read::ReadPolicy;
use bluetti_controller::transport::station::{self, Poll};

use super::{Context, detect, session};

pub async fn run(ctx: &Context<'_>, target: &str) -> anyhow::Result<()> {
    session::run(ctx, target, async |gatt, advert| {
        let model = advert
            .name
            .as_deref()
            .and_then(Model::from_ble_name)
            .ok_or_else(|| anyhow!("unknown model, no register profile to read"))?;
        let policy = ReadPolicy::from_ble(ctx.ble);
        let identity = station::detect(gatt, advert, &policy).await?;
        println!("{}", detect::identity_line(&identity));
        let profile = model.profile();
        let poll = station::poll(gatt, profile, &policy).await?;
        for line in report(profile, &poll) {
            println!("{line}");
        }
        verdict(&poll)
    })
    .await
}

/// One line per profile field (value or `-`, unit, `verified`), then one per
/// problem.
fn report(profile: &Profile, poll: &Poll) -> Vec<String> {
    let fields = profile.fields.iter().map(|field| {
        let value = poll
            .values
            .get(&field.metric)
            .map_or("-".to_owned(), |value| format!("{value:.2}"));
        format!(
            "{:<20} {value:>9} {:<2}  verified={}",
            field.metric.key(),
            field.metric.unit(),
            field.verified
        )
    });
    let problems = poll.problems.iter().map(|problem| match problem {
        Problem::Missing { metric } => format!("problem: {} missing", metric.key()),
        Problem::OutOfRange { metric, value } => {
            let range = metric.range();
            format!(
                "problem: {} = {value} {} outside {}..={}",
                metric.key(),
                metric.unit(),
                range.start(),
                range.end()
            )
        }
    });
    fields.chain(problems).collect()
}

/// A poll with any problem fails, so the process exits non-zero.
fn verdict(poll: &Poll) -> anyhow::Result<()> {
    match poll.problems.len() {
        0 => Ok(()),
        n => bail!("{n} field(s) missing or implausible"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bluetti_controller::protocol::device::Metric;

    use super::*;

    fn poll(problems: Vec<Problem>) -> Poll {
        Poll {
            values: BTreeMap::from([(Metric::BatterySoc, 87.0), (Metric::AcOutputPower, 42.0)]),
            problems,
        }
    }

    #[test]
    fn report_prints_every_field_then_each_problem() {
        let poll = poll(vec![Problem::OutOfRange {
            metric: Metric::PackVoltage,
            value: 655.35,
        }]);
        let lines = report(Model::Eb3a.profile(), &poll);
        assert_eq!(lines.len(), Model::Eb3a.profile().fields.len() + 1);
        // EB3A battery_soc is verified (HUMAN-CHECKLIST H1, 2026-10-15).
        assert!(lines.contains(&format!(
            "{:<20} {:>9} {:<2}  verified=true",
            "battery_soc", "87.00", "%"
        )));
        assert!(lines.contains(&format!(
            "{:<20} {:>9} {:<2}  verified=false",
            "dc_input_power", "-", "W"
        )));
        assert_eq!(
            lines.last().unwrap(),
            "problem: pack_voltage = 655.35 V outside 0..=60"
        );
    }

    #[test]
    fn any_problem_fails_the_read() {
        assert!(verdict(&poll(Vec::new())).is_ok());
        let err = verdict(&poll(vec![Problem::Missing {
            metric: Metric::BatterySoc,
        }]))
        .unwrap_err();
        assert_eq!(err.to_string(), "1 field(s) missing or implausible");
    }
}

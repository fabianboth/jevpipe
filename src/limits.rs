use std::fmt;
use std::future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{self, Instant};

use crate::cost::Cost;
use crate::service::{Exhausted, Usage};
use crate::settings::{Limit, Settings};
use crate::tokens::Tokens;

pub(crate) struct Limits {
    spend: Meter,
    tokens: Meter,
    deadline: Limit<Deadline>,
    stop: watch::Sender<Option<Stop>>,
}

struct Meter {
    measure: Measure,
    total: AtomicU64,
    reported: AtomicBool,
    limit: Limit<u64>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Measure {
    Spend,
    Tokens,
}

#[derive(Clone, Copy)]
struct Deadline {
    at: Instant,
    limit: Duration,
}

#[derive(Clone, Copy)]
pub(crate) enum Stop {
    Limit { measure: Measure, limit: u64 },
    Time { limit: Duration },
    Exhausted(Exhausted),
}

pub(crate) struct StopLine {
    reason: Reason,
    resume_line: usize,
}

enum Reason {
    Reached {
        measure: Measure,
        limit: u64,
        used: u64,
    },
    Time {
        limit: Duration,
    },
    Exhausted(Exhausted),
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Reported {
    pub(crate) cost: Option<Cost>,
    pub(crate) tokens: Option<Tokens>,
}

#[derive(Debug, thiserror::Error)]
#[error("the service reported no {}, so {} cannot be enforced", .0.unreported(), .0.flag())]
pub(crate) struct Unreported(Measure);

#[derive(Debug, thiserror::Error)]
#[error("--max-time {} is too long", humantime::format_duration(*.0))]
pub(crate) struct TooLong(Duration);

impl Limits {
    pub(crate) fn new(settings: &Settings) -> Result<Self, TooLong> {
        let deadline = match settings.max_time {
            Limit::At(limit) => Limit::At(Deadline {
                at: Instant::now().checked_add(limit).ok_or(TooLong(limit))?,
                limit,
            }),
            Limit::Unlimited => Limit::Unlimited,
        };
        Ok(Self {
            spend: Meter::new(Measure::Spend, settings.max_cost.map(Cost::nanos)),
            tokens: Meter::new(Measure::Tokens, settings.max_tokens.map(Tokens::count)),
            deadline,
            stop: watch::Sender::new(None),
        })
    }

    pub(crate) fn may_send(&self) -> bool {
        if self.is_stopped() {
            return false;
        }
        match self.meters().into_iter().find_map(Meter::reached) {
            Some(stop) => {
                self.stop(stop);
                false
            }
            None => true,
        }
    }

    pub(crate) fn add(&self, usage: &Usage) -> Result<(), Unreported> {
        self.spend.add(usage.cost.map(Cost::nanos))?;
        self.tokens.add(usage.tokens.map(Tokens::count))
    }

    pub(crate) fn stop(&self, stop: Stop) {
        self.stop.send_if_modified(|current| {
            let first = current.is_none();
            current.get_or_insert(stop);
            first
        });
    }

    pub(crate) async fn until_stopped(&self) {
        let _ = self.stop.subscribe().wait_for(Option::is_some).await;
    }

    pub(crate) async fn until_deadline(&self) {
        match self.deadline {
            Limit::At(deadline) => {
                time::sleep_until(deadline.at).await;
                self.stop(Stop::Time {
                    limit: deadline.limit,
                });
            }
            Limit::Unlimited => future::pending().await,
        }
    }

    pub(crate) fn reported(&self) -> Reported {
        Reported {
            cost: self.spend.reported().map(Cost::from_nanos),
            tokens: self.tokens.reported().map(Tokens::from_count),
        }
    }

    pub(crate) fn stop_line(&self, resume_line: usize) -> Option<StopLine> {
        let reason = match (*self.stop.borrow())? {
            Stop::Limit { measure, limit } => Reason::Reached {
                measure,
                limit,
                used: self.meter(measure).total(),
            },
            Stop::Time { limit } => Reason::Time { limit },
            Stop::Exhausted(exhausted) => Reason::Exhausted(exhausted),
        };
        Some(StopLine {
            reason,
            resume_line,
        })
    }

    fn meters(&self) -> [&Meter; 2] {
        [&self.spend, &self.tokens]
    }

    fn meter(&self, measure: Measure) -> &Meter {
        match measure {
            Measure::Spend => &self.spend,
            Measure::Tokens => &self.tokens,
        }
    }

    fn is_stopped(&self) -> bool {
        self.stop.borrow().is_some()
    }
}

impl Meter {
    const fn new(measure: Measure, limit: Limit<u64>) -> Self {
        Self {
            measure,
            total: AtomicU64::new(0),
            reported: AtomicBool::new(false),
            limit,
        }
    }

    fn add(&self, amount: Option<u64>) -> Result<(), Unreported> {
        match (amount, self.limit) {
            (Some(amount), _) => {
                self.total.fetch_add(amount, Ordering::Relaxed);
                self.reported.store(true, Ordering::Relaxed);
                Ok(())
            }
            (None, Limit::At(_)) => Err(Unreported(self.measure)),
            (None, Limit::Unlimited) => Ok(()),
        }
    }

    fn reached(&self) -> Option<Stop> {
        match self.limit {
            Limit::At(limit) if self.total() >= limit => Some(Stop::Limit {
                measure: self.measure,
                limit,
            }),
            Limit::At(_) | Limit::Unlimited => None,
        }
    }

    fn reported(&self) -> Option<u64> {
        self.reported.load(Ordering::Relaxed).then(|| self.total())
    }

    fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }
}

impl Measure {
    const fn flag(self) -> &'static str {
        match self {
            Self::Spend => "--max-cost",
            Self::Tokens => "--max-tokens",
        }
    }

    const fn unreported(self) -> &'static str {
        match self {
            Self::Spend => "cost",
            Self::Tokens => "token count",
        }
    }
}

impl fmt::Display for StopLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("stopped: ")?;
        match self.reason {
            Reason::Reached {
                measure: Measure::Spend,
                limit,
                used,
            } => write!(
                formatter,
                "spend limit {} reached ({} spent)",
                Cost::from_nanos(limit),
                Cost::from_nanos(used)
            )?,
            Reason::Reached {
                measure: Measure::Tokens,
                limit,
                used,
            } => write!(formatter, "token limit {limit} reached ({used} used)")?,
            Reason::Time { limit } => write!(
                formatter,
                "time limit {} reached",
                humantime::format_duration(limit)
            )?,
            Reason::Exhausted(exhausted) => write!(formatter, "{exhausted}")?,
        }
        write!(
            formatter,
            "; input from line {} on was not processed",
            self.resume_line
        )
    }
}

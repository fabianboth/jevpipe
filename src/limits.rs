use std::fmt;
use std::future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{self, Instant};

use crate::cost::Cost;
use crate::service::Exhausted;
use crate::settings::{Limit, Settings};

pub(crate) struct Limits {
    spent: AtomicU64,
    costed: AtomicBool,
    max_cost: Limit<Cost>,
    max_time: Limit<Duration>,
    started: Instant,
    stop: watch::Sender<Option<Stop>>,
}

#[derive(Clone, Copy)]
pub(crate) enum Stop {
    Spend { limit: Cost },
    Time { limit: Duration },
    Exhausted(Exhausted),
}

pub(crate) struct StopLine {
    stop: Stop,
    spent: Cost,
    resume_line: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("the service reported no cost, so --max-cost cannot be enforced")]
pub(crate) struct NoCost;

impl Limits {
    pub(crate) fn new(settings: &Settings) -> Self {
        Self {
            spent: AtomicU64::new(0),
            costed: AtomicBool::new(false),
            max_cost: settings.max_cost,
            max_time: settings.max_time,
            started: Instant::now(),
            stop: watch::Sender::new(None),
        }
    }

    pub(crate) fn may_send(&self) -> bool {
        if self.is_stopped() {
            return false;
        }
        match self.max_cost {
            Limit::At(limit) if self.spent() >= limit => {
                self.stop(Stop::Spend { limit });
                false
            }
            Limit::At(_) | Limit::Unlimited => true,
        }
    }

    pub(crate) fn add(&self, cost: Option<Cost>) -> Result<(), NoCost> {
        match (cost, self.max_cost) {
            (Some(cost), _) => {
                self.spent.fetch_add(cost.nanos(), Ordering::Relaxed);
                self.costed.store(true, Ordering::Relaxed);
                Ok(())
            }
            (None, Limit::At(_)) => Err(NoCost),
            (None, Limit::Unlimited) => Ok(()),
        }
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
        match self.max_time {
            Limit::At(limit) => {
                time::sleep_until(self.started + limit).await;
                self.stop(Stop::Time { limit });
            }
            Limit::Unlimited => future::pending().await,
        }
    }

    pub(crate) fn reported_cost(&self) -> Option<Cost> {
        self.costed.load(Ordering::Relaxed).then(|| self.spent())
    }

    pub(crate) fn stop_line(&self, resume_line: usize) -> Option<StopLine> {
        let stop = (*self.stop.borrow())?;
        Some(StopLine {
            stop,
            spent: self.spent(),
            resume_line,
        })
    }

    fn is_stopped(&self) -> bool {
        self.stop.borrow().is_some()
    }

    fn spent(&self) -> Cost {
        Cost::from_nanos(self.spent.load(Ordering::Relaxed))
    }
}

impl fmt::Display for StopLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("stopped: ")?;
        match self.stop {
            Stop::Spend { limit } => {
                write!(
                    formatter,
                    "spend limit {limit} reached ({} spent)",
                    self.spent
                )?;
            }
            Stop::Time { limit } => {
                write!(
                    formatter,
                    "time limit {} reached",
                    humantime::format_duration(limit)
                )?;
            }
            Stop::Exhausted(exhausted) => write!(formatter, "{exhausted}")?,
        }
        write!(
            formatter,
            "; input from line {} on was not processed",
            self.resume_line
        )
    }
}

use std::fmt;
use std::str::FromStr;

use unit_prefix::NumberPrefix;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Tokens {
    count: u64,
}

#[derive(Debug, thiserror::Error)]
#[error("not a count, for example 250k or 5M")]
pub(crate) struct NotACount;

pub(crate) struct Abbreviated(Tokens);

impl Tokens {
    pub(crate) const fn from_count(count: u64) -> Self {
        Self { count }
    }

    pub(crate) const fn count(self) -> u64 {
        self.count
    }

    pub(crate) const fn is_zero(self) -> bool {
        self.count == 0
    }

    pub(crate) const fn abbreviated(self) -> Abbreviated {
        Abbreviated(self)
    }
}

impl FromStr for Tokens {
    type Err = NotACount;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_size::Config::new()
            .parse_size(text)
            .map(Self::from_count)
            .map_err(|_| NotACount)
    }
}

impl fmt::Display for Tokens {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.count)
    }
}

impl fmt::Display for Abbreviated {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match NumberPrefix::decimal(self.0.count as f64) {
            NumberPrefix::Standalone(_) => write!(formatter, "{}", self.0.count),
            NumberPrefix::Prefixed(prefix, value) => write!(formatter, "{value:.1}{prefix}"),
        }
    }
}

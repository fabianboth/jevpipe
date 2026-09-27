use std::fmt;
use std::iter;
use std::str::FromStr;

const NANOS_PER_DOLLAR: u64 = 1_000_000_000;
const NANOS_PER_SHOWN_UNIT: u64 = 1_000;
const DECIMALS: usize = 9;
const SHOWN_DECIMALS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Cost {
    nanos: u64,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CostError {
    #[error("not an amount of US dollars, for example 0.5 or 2")]
    NotAnAmount,
    #[error("more than {DECIMALS} decimals")]
    TooPrecise,
    #[error("too large")]
    TooLarge,
}

impl Cost {
    pub(crate) const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    pub(crate) const fn nanos(self) -> u64 {
        self.nanos
    }

    pub(crate) const fn is_zero(self) -> bool {
        self.nanos == 0
    }

    pub(crate) fn from_dollars(dollars: f64) -> Result<Self, CostError> {
        format!("{dollars:.DECIMALS$}").parse()
    }
}

impl FromStr for Cost {
    type Err = CostError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if (whole.is_empty() && fraction.is_empty()) || !digits(whole) || !digits(fraction) {
            return Err(CostError::NotAnAmount);
        }
        let missing_decimals = DECIMALS
            .checked_sub(fraction.len())
            .ok_or(CostError::TooPrecise)?;
        let whole: u64 = match whole {
            "" => 0,
            digits => digits.parse().map_err(|_| CostError::TooLarge)?,
        };
        let fraction: u64 = match fraction {
            "" => 0,
            digits => digits.parse().map_err(|_| CostError::NotAnAmount)?,
        };
        let scale: u64 = iter::repeat_n(10, missing_decimals).product();
        whole
            .checked_mul(NANOS_PER_DOLLAR)
            .and_then(|nanos| nanos.checked_add(fraction * scale))
            .map(Self::from_nanos)
            .ok_or(CostError::TooLarge)
    }
}

impl fmt::Display for Cost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown = self.nanos.saturating_add(NANOS_PER_SHOWN_UNIT / 2) / NANOS_PER_SHOWN_UNIT;
        let per_dollar = NANOS_PER_DOLLAR / NANOS_PER_SHOWN_UNIT;
        let fraction = format!("{:0SHOWN_DECIMALS$}", shown % per_dollar);
        match fraction.trim_end_matches('0') {
            "" => write!(formatter, "${}", shown / per_dollar),
            fraction => write!(formatter, "${}.{fraction}", shown / per_dollar),
        }
    }
}

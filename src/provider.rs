use std::fmt;
use std::str::FromStr;

const OPENROUTER: &str = "openrouter";
const TYPESAFE: &str = "typesafe";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Provider {
    #[default]
    OpenRouter,
    TypeSafe,
}

#[derive(Debug, thiserror::Error)]
#[error("the providers are {}", Provider::ALL.map(Provider::id).join(", "))]
pub(crate) struct UnknownProvider;

impl Provider {
    pub(crate) const ALL: [Self; 2] = [Self::OpenRouter, Self::TypeSafe];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::OpenRouter => "OpenRouter",
            Self::TypeSafe => "TypeSafe",
        }
    }

    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::OpenRouter => OPENROUTER,
            Self::TypeSafe => TYPESAFE,
        }
    }

    pub(crate) const fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenRouter => "https://openrouter.ai/api",
            Self::TypeSafe => "https://api.typesafe.ai",
        }
    }

    pub(crate) const fn key_variable(self) -> &'static str {
        match self {
            Self::OpenRouter => "OPENROUTER_API_KEY",
            Self::TypeSafe => "TYPESAFE_API_KEY",
        }
    }

    pub(crate) const fn keychain_user(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter-api-key",
            Self::TypeSafe => "typesafe-api-key",
        }
    }

    pub(crate) const fn reports_cost(self) -> bool {
        match self {
            Self::OpenRouter => true,
            Self::TypeSafe => false,
        }
    }

    pub(crate) const fn other(self) -> Self {
        match self {
            Self::OpenRouter => Self::TypeSafe,
            Self::TypeSafe => Self::OpenRouter,
        }
    }
}

impl FromStr for Provider {
    type Err = UnknownProvider;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|provider| provider.id() == text)
            .ok_or(UnknownProvider)
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

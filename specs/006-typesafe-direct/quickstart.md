# Quickstart: TypeSafe's Own API as a Second Provider

Manual checks after implementation, with real keys (the automated tests need none). Use a throwaway
config (`$env:JEVPIPE_CONFIG = "$env:TEMP\jevpipe-006.toml"` or `export JEVPIPE_CONFIG=/tmp/jevpipe-006.toml`).
Every run below costs well under a cent.

## 1. TypeSafe with the environment variable (User Story 1)

```sh
jevpipe config set provider typesafe
git log --oneline -20 | jevpipe filter "Does this commit fix a bug?"
```

The expected output:
- the kept commits;
- a summary ending in `… tokens, …s`;
- exit status 0 or 1.

`jevpipe config list` shows `provider = "typesafe"`, `base-url = "https://api.typesafe.ai"  # default` and
`# API key: from TYPESAFE_API_KEY`.

## 2. TypeSafe with the keychain

```sh
jevpipe auth set-key          # paste the TypeSafe key at "TypeSafe API key: "
TYPESAFE_API_KEY= git log --oneline -5 | jevpipe filter "Does this commit fix a bug?"
jevpipe config list           # "# API key: from the keychain"
```

The OpenRouter key stored earlier is untouched: `jevpipe config unset provider` then a run works
with it.

## 3. Wrong model and a missing key

```sh
git log --oneline -1 | jevpipe filter --model jev-1.13 "Is it?"
```

Expected: `service error: 400 Bad Request: Unknown model: jev-1.13`, exit 2.

With the provider unset and no OpenRouter key, a run fails naming the provider and
`jevpipe config set provider typesafe`.

## 4. Limits on TypeSafe (User Story 3)

```sh
git log --oneline -200 | jevpipe map --max-tokens 5k -q '{"fix":{"type":"noul","instructions":"Does this commit fix a bug?"}}'
git log --oneline -5 | jevpipe filter --max-cost 0.1 "Is it?"
```

Expected:
- The first run stops with `token limit 5000 reached (… used); input from line L on …`, exit status 3.
- The second run fails before any request, pointing to `--max-tokens`, exit status 2.

## 5. Settings per provider (User Story 4)

```sh
jevpipe config set max-cost 0.5            # top level
jevpipe config set provider typesafe
git log --oneline -5 | jevpipe filter "Is it?"   # refused: shows the two commands below
jevpipe config set openrouter.max-cost 0.5
jevpipe config unset max-cost
jevpipe config set typesafe.max-tokens 5M
jevpipe config set typesafe.model jev-1.13.0
jevpipe config set openrouter.model jev-1.13
git log --oneline -5 | jevpipe filter "Is it?"   # TypeSafe, jev-1.13.0, token limit
jevpipe config list                              # origins: config file [typesafe]
```

## 6. OpenRouter unchanged (SC-002)

```sh
jevpipe config unset provider                    # uses [openrouter]: max-cost 0.5, jev-1.13
git log --oneline -20 | jevpipe filter "Does this commit fix a bug?"
```

The summary ends in `… tokens, $0.0000…, …s`. `--max-tokens 5k` stops it the same way as on
TypeSafe.

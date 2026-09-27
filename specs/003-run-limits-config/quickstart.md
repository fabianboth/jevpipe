# Quickstart: Run Limits, User Config and a Stored API Key

## Verify (offline, no API key, no keychain, no user config)

```powershell
./check.ps1 -Fix
./check.ps1 -Filter limit
./check.ps1 -Filter config
```

## Try it against the real service

```bash
cargo build --release
J=./target/release/jevpipe

# store the key once (hidden prompt), then no export needed in new shells
$J auth set-key
$J config list                      # "# API key: from the keychain"

# or piped, e.g. from a password manager
echo "$OPENROUTER_API_KEY" | $J auth set-key

# defaults once
$J config set max-cost 0.50
$J config set request-timeout 20s
$J config path
$J config list

# a guarded run: stops at 2 cents or 30 seconds, whichever comes first
git ls-files | $J map -f triage.json --read-files --max-cost 0.02 --max-time 30s > out.jsonl
echo $?                             # 3 when a limit stopped it

# continue where it stopped (line L from "input from line L on was not processed")
git ls-files | tail -n +L | $J map -f triage.json --read-files >> out.jsonl

# lift the configured limit for one run
$J filter "Is this line an error?" app.log --max-cost none

# clean up
$J config unset max-cost
$J auth remove-key
```

Check after each run: the stop line and the summary on stderr, `echo $?` (3 = stopped by a limit),
and that no key appears anywhere in the output.

## Manual checks (cannot run in CI)

Hidden prompt, on a real terminal (typing `abc`, then Enter; then again with Ctrl+C):

| Terminal | Expected |
|---|---|
| Windows Terminal / VS Code terminal, PowerShell, cmd | prompt, typing hidden; Ctrl+C prints `interrupted`, exit 130, typing visible again at the shell prompt without an extra Enter |
| Git Bash window (mintty without pseudo console) | prompt says the input will be visible; key read; no hang |
| macOS Terminal, a Linux terminal | prompt, typing hidden; Ctrl+C restores echo, exit 130 |

Keychain, per platform (set, run without `OPENROUTER_API_KEY`, remove):

| Platform | Expected |
|---|---|
| Windows | entry `openrouter-api-key.jevpipe` in Credential Manager |
| macOS | `security find-generic-password -s jevpipe -a openrouter-api-key` finds it; after `cargo build --release` of a changed jevpipe, the key still reads without any dialog |
| Linux desktop | entry in the login keyring (Seahorse / KWallet) |
| Linux without Secret Service (WSL, container) | `auth set-key` fails with "no keychain available here …; set OPENROUTER_API_KEY instead"; runs with the variable work |

Spike results these checks repeat are recorded in [research.md](research.md) R7–R9.

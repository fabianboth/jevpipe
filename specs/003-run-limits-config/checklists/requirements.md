# Specification Quality Checklist: Run Limits, User Config and a Stored API Key

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-27
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- As in the earlier milestones, the user surface of a CLI is its flags, commands, exit statuses,
  environment variables and file locations; naming them is the product's behaviour, not an
  implementation detail. Crates, the stand-in keychain mechanism and code structure are left to the plan.
- TOML is named as the config format because users edit the file by hand.
- Keychain spike done (2026-09-27): Windows and Linux (with and without a Secret Service) work;
  macOS native access blocks after a rebuild, so macOS goes through the system keychain tool
  (verified on a macOS runner without any dialog). Hidden input on standard input verified in the
  Windows console, PowerShell, cmd, Git Bash (visible fallback) and a Linux pseudo-terminal, including
  Ctrl+C restoring the terminal.
- Decided with the user (2026-09-27): the Linux Secret Service stays in scope despite its dependency
  weight (about 73 extra crates, Linux only); the macOS key-storage trade-off is accepted; exit status 3
  outranks 2; `--help` shows configured defaults; `none` lifts a configured limit; durations require a
  unit and use a standard parser; the CLAUDE.md testing rule is generalised rather than given an
  exception. No open questions remain.

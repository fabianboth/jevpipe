# jevpipe Constitution

## Core Principles

### I. Lean MVP approach
Focus on a core set of features creating a clean but not overengineered solution. Avoid feature creep; do one thing well. If a feature doesn't directly contribute to the core value proposition, it should be deferred.

### II. Automated Verification
All changes must pass automated checks (format, lint, test). New features should include tests where applicable. Tests run offline against a stub of the API. We value confidence in our releases; automation is the key to maintaining velocity without sacrificing quality.

### III. Reusable Components
Prefer reusing existing components and code, creating well maintainable, modular and reusable code with good interfaces.

### IV. Decide, Don't Act
jevpipe answers typed questions (yes/no, pick one, score) and nothing else: no planning, no text generation, no executing actions. It is stateless; the calling script owns history, planning and acting.

### V. A Good Unix Citizen
Plain lines or JSONL in, JSONL out that keeps each record's id, so it composes with `jq`, `head` and `xargs`. Safe to run unattended: budgets on records, spend and time, clean exit codes, a one-line summary on stderr. Cheap to rerun and fast: identical requests are cached, startup is near zero.

**Version**: 1.0.0 | **Ratified**: 2026-09-26

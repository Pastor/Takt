## General Principles

- Reason from first principles — question assumptions before adopting patterns.
- Follow Clean Code, DRY, and KISS — favor clarity over cleverness.
- Don't over-abstract and keep it simple.
- Practice TDD: write a failing test first, then implement the minimal code to make it pass, then refactor.

## Development

- Run `precheck.sh` before committing to verify code quality and formatting.
- Localize all user-facing strings — no hardcoded display text in source files.
- Follow the guidelines in [Testing](./doc/TESTING.md) for test structure, naming, and coverage expectations.
- Group the changes logically, but not more than 300 lines of code, and create a patch file (in directory `changes`)
  with naming `Changes-XX[-PYY].patch` where `XX` sequence number of change and `-PYY` postfix addon if necessary when
  large changes and `YY` is part of sequence number.

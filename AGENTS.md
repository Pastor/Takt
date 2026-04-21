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
- Append change log to `CHANGES.md` file on top of the list.
  Using [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format and [Semantic Versioning](https://semver.org/).
- Use meaningful commit messages that describe the changes made.
- Use consistent naming conventions and follow established coding standards.
- Document your code with comments and docstrings to improve readability and maintainability.
- Using Russian language in comments, change log, commit messages and docstrings.

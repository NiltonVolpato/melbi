# MELBI
- Melbi is a safe, fast, embeddable expression language.
- The entire program is a single expression.
- See @docs/melbi-lang-cheat-sheet.md for a quick syntax reference.

# CRITICAL SAFETY GUIDELINES
- **NEVER run without asking first:**
  - `git checkout` - You don't know when the last commit was. This has lost hours of work.
  - `perl` or complex `sed` - You WILL get replacements wrong.
    - Use `sed` only for simple single-line substitutions with `-i.bkp` backups.
  - File deletions, git operations (reset, stash), bulk find/replace
- **If anything goes wrong, STOP immediately:**
  - Don't run more commands to "undo" damage
  - Don't try to restore from memory
  - STOP and ask the user for help

# CODING GUIDELINES
- Do not use `unsafe` or `transmute` without asking first.
  - Permission applies only to that specific instance.
  - Document safety invariants thoroughly.

# TESTING GUIDELINES
- Think about good test cases covering normal and corner cases.
- Test for success and failure scenarios.
  - Success scenarios should validate the answer.
  - Failure scenarios should validate the expected error kind and other relevant details.
- Write high-level tests before implementing the code.
- **NEVER REMOVE A FAILING TEST:**
  - If a test fails, do not remove or modify it.
  - Your job is to find failing tests and bugs, and not hide them!
  - Ask the user for help if you think you found a bug or if the test had wrong assumptions.

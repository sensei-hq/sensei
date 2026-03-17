# sensei — Agent Orientation

## Goals
This repo is indexed by sensei. Start every session with `get_session_context()`.

## Stack
- typescript

## Guidelines
- Call `search(query)` to find symbols — do not grep or explore manually
- Call `load_context(file_path)` to read a file with its extracted symbols
- Call `get_session_context()` if you are unsure of your current task

## Patterns
See `.sensei/patterns.md` for project-specific conventions.

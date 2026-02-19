# LLM Provider Conventions

This project keeps `src/llm.rs` model-agnostic.

## Rules

- Do not branch on specific model name strings in provider send/translation paths.
- Represent provider-specific behavior as capability flags and branch on capabilities.
- Keep provider/model presets in setup/config surfaces; keep `llm.rs` focused on protocol translation and runtime behavior.
- Write tests around capability combinations rather than model-name cases.

## Why

- Reduces model-churn edits in core runtime code.
- Makes behavior portable across providers exposing similar capabilities.
- Keeps compatibility logic explicit, testable, and easier to maintain.

---
description: Review git changes, summarize them clearly, ask for confirmation via options, then commit.
---

Run `git status` and `git diff --stat` to see the current state. If there
are changes, present a concise one-line summary per changed area. Then use
the question tool with options to ask the user whether to commit. Do NOT
commit unless the user explicitly confirms via the options. When the user
confirms, stage and commit with a clear message.

$ARGUMENTS

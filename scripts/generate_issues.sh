#!/bin/bash

# Ensure gh CLI is installed
if ! command -v gh &> /dev/null
then
    echo "gh CLI could not be found. Please install it to run this script."
    exit 1
fi

# Issue 1: redundant_env_clone
gh issue create \
  --title "Implement \`redundant_env_clone\` lint" \
  --body "## Summary
Flags calling \`.clone()\` on the \`Env\` object since it is designed to be passed cheaply by reference or copy value.

## Tech Stack
- Rust
- Dylint / rustc AST

## Acceptance Criteria
- [ ] Lint correctly identifies \`env.clone()\`
- [ ] UI tests written and passing
- [ ] Emits a warning by default
" \
  --label "enhancement,lint"

# Issue 2: unnecessary_host_function_call
gh issue create \
  --title "Implement \`unnecessary_host_function_call\` lint" \
  --body "## Summary
Identifies redundant calls to host functions (like fetching the ledger sequence) inside loop bodies.

## Tech Stack
- Rust
- Dylint / rustc AST

## Acceptance Criteria
- [ ] Lint identifies repeated host function calls in loops
- [ ] UI tests written and passing
- [ ] Emits a warning by default
" \
  --label "enhancement,lint"

# Issue 3: CLI Wrapper
gh issue create \
  --title "Develop \`cargo-cost-lint\` CLI Wrapper" \
  --body "## Summary
Create a custom Cargo subcommand wrapper to orchestrate Dylint under the hood for better UX.

## Tech Stack
- Rust (clap/argh)

## Acceptance Criteria
- [ ] \`cargo cost-lint\` command works in a workspace
- [ ] Successfully calls dylint
- [ ] Handles exit codes properly
" \
  --label "enhancement,cli"

# Issue 4: Docs Site
gh issue create \
  --title "Setup Documentation Site (mdBook or GitBook)" \
  --body "## Summary
Create a dedicated documentation site explaining the lints and setup instructions.

## Tech Stack
- mdBook

## Acceptance Criteria
- [ ] Site initialized
- [ ] Lint reference pages added
- [ ] Setup instructions added
" \
  --label "documentation"

echo "Issues generated successfully!"

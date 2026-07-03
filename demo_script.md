# Demo Video Script

**Target Length:** 1 - 2 minutes
**Goal:** Demonstrate `soroban-cost-linter` catching an expensive anti-pattern, blocking a pipeline, and then passing once fixed.

### Scene 1: The Problem (0:00 - 0:20)
*   **Visual:** Terminal split screen or IDE (VS Code). Show a basic Soroban contract (`src/lib.rs`).
*   **Action:** Highlight a `for` loop that calls `env.storage().instance().set(&i, &1)`.
*   **Voiceover:** "When building on Soroban, calling storage operations inside loops creates a massive, input-independent drain on your transaction budget. `soroban-cost-linter` is built to catch these structural flaws."

### Scene 2: Running the Linter (0:20 - 0:45)
*   **Action:** Open the terminal in the workspace root. Run `cargo cost-lint`.
*   **Visual:** The linter runs and immediately outputs a standard compiler-style error block.
*   **Visual:** Show the error message: `error: storage operation inside a loop`, pointing exactly to the `.set()` call. The exit code is non-zero, proving it can block CI/CD pipelines.

### Scene 3: Fixing the Code (0:45 - 1:10)
*   **Action:** In the code editor, move the `.set()` call *outside* the loop. (e.g., aggregate state in a `Map`, loop to populate the map, then do one `.set()` at the end).
*   **Voiceover:** "To fix this, we aggregate our state changes in memory and execute a single storage write after the loop."

### Scene 4: The Pass & Conclusion (1:10 - 1:30)
*   **Action:** Run `cargo cost-lint` again.
*   **Visual:** The command completes silently/successfully.
*   **Voiceover:** "The code is now structurally safe. By adding `soroban-cost-linter` to your CI pipeline alongside Tollcraft's runtime tools, you ensure your contracts remain highly optimized."

#!/bin/bash
# Continuous development script for RustOS

cd /home/k/Desktop/aider/rusteze

echo "Starting RustOS continuous development..."
echo "Press Ctrl+C to stop"
echo ""

SESSION=0

while true; do
    SESSION=$((SESSION + 1))
    COMMITS=$(git rev-list --count HEAD 2>/dev/null || echo "0")
    DONE=$(grep -c "\[x\]" AIDER_INSTRUCTIONS.md 2>/dev/null || echo "0")
    TODO=$(grep -c "\[ \]" AIDER_INSTRUCTIONS.md 2>/dev/null || echo "0")

    # Get next 3 unchecked items
    NEXT_TASKS=$(grep -m3 "\[ \]" AIDER_INSTRUCTIONS.md | sed 's/- \[ \] /  - /')

    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║ Session: $SESSION | $(date '+%Y-%m-%d %H:%M:%S')"
    echo "║ Commits: $COMMITS | Done: $DONE | Todo: $TODO"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""

    # Find all .rs files and add them
    RS_FILES=$(find src -name "*.rs" 2>/dev/null | tr '\n' ' ')

    aider $RS_FILES \
        AIDER_INSTRUCTIONS.md \
        Cargo.toml \
        --message "
Read AIDER_INSTRUCTIONS.md. Work through unchecked [ ] items.

NEXT TASKS:
$NEXT_TASKS

CRITICAL: After EVERY change, run:
  RUSTFLAGS=\"-D warnings\" cargo build --release

Warnings are ERRORS. Code must compile with ZERO warnings before marking [x].

WORKFLOW:
1. Implement feature (create new .rs file if needed)
2. Update lib.rs to include the module
3. RUN THE BUILD - do not skip this step
4. Fix ALL errors and warnings
5. Only mark [x] when build succeeds with no warnings
6. Move to next task

Use WHOLE edit format - output complete file contents.
"

    EXIT_CODE=$?

    COMMITS_AFTER=$(git rev-list --count HEAD 2>/dev/null || echo "0")
    NEW_COMMITS=$((COMMITS_AFTER - COMMITS))

    echo ""
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│ Session $SESSION complete (exit: $EXIT_CODE)"
    echo "│ New commits: $NEW_COMMITS | Total: $COMMITS_AFTER"
    echo "└─────────────────────────────────────────────────────────────┘"

    if [ $NEW_COMMITS -gt 0 ]; then
        echo ""
        echo "Recent commits:"
        git log --oneline -n $NEW_COMMITS
        echo ""
        echo "Pushing to origin..."
        git push origin master
    fi

    # Only restart if there's more work to do
    if [ "$TODO" -eq 0 ]; then
        echo "All tasks complete!"
        break
    fi

    echo ""
    sleep 1
done

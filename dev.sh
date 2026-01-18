#!/bin/bash
# Continuous development script for RustOS

cd /home/k/Desktop/aider/rusteze

echo "Starting RustOS continuous development..."
echo "Press Ctrl+C to stop"
echo ""

SESSION=0
STUCK_COUNT=0

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

    # If aider crashed (non-zero exit), restart ollama to clear VRAM
    if [ $EXIT_CODE -ne 0 ]; then
        echo ""
        echo "Aider exited with error ($EXIT_CODE). Restarting ollama..."
        pkill -f "ollama serve"
        sleep 3
        ollama serve &>/dev/null &
        sleep 5
        # Warm up the model
        echo "Warming up model..."
        curl -s http://localhost:11434/api/generate -d '{"model":"qwen3-30b-aider:latest","prompt":"hi","stream":false}' > /dev/null 2>&1
        sleep 2
    fi

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
        STUCK_COUNT=0
    else
        STUCK_COUNT=$((STUCK_COUNT + 1))
        echo "No progress made (stuck count: $STUCK_COUNT)"

        if [ $STUCK_COUNT -ge 2 ]; then
            echo ""
            echo "Calling Claude Code for help..."
            BUILD_OUTPUT=$(RUSTFLAGS="-D warnings" cargo build --release 2>&1 | tail -30)
            claude --print "
The local AI (aider with qwen3-30b) is stuck on this project. Please help.

Current task from AIDER_INSTRUCTIONS.md:
$NEXT_TASKS

Last build output:
$BUILD_OUTPUT

Please:
1. Read the relevant source files
2. Fix any issues preventing progress
3. Run the build to verify
4. Update AIDER_INSTRUCTIONS.md if task is complete
"
            STUCK_COUNT=0
        fi
    fi

    # Only restart if there's more work to do
    if [ "$TODO" -eq 0 ]; then
        echo "All tasks complete!"
        break
    fi

    echo ""
    sleep 1
done

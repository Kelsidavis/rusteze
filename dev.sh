#!/bin/bash
# Continuous development script for RustOS

cd "$(dirname "$0")"

# Cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    pkill -9 -f "ollama" 2>/dev/null
    exit 0
}
trap cleanup SIGINT SIGTERM

# Use only the RTX 5080 (GPU 1)
export CUDA_VISIBLE_DEVICES=GPU-707f560b-e5d9-3fea-9af2-c6dd2b77abbe
export OLLAMA_FLASH_ATTENTION=1
export OLLAMA_KV_CACHE_TYPE=q8_0
export OLLAMA_NUM_CTX=12288

echo "Starting RustOS continuous development..."
echo "Press Ctrl+C to stop"
echo ""

# Kill any zombie ollama processes before starting
echo "Cleaning up any existing ollama processes..."
pkill -9 -f "ollama" 2>/dev/null
sleep 2

# Verify they're dead
while pgrep -f "ollama" >/dev/null 2>&1; do
    echo "Waiting for ollama processes to terminate..."
    pkill -9 -f "ollama" 2>/dev/null
    sleep 1
done

# Start fresh ollama instance
echo "Starting ollama..."
ollama serve &>/dev/null &
sleep 3

# Wait for ollama to be ready
echo "Waiting for ollama to be ready..."
for i in {1..30}; do
    if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
        echo "Ollama is ready."
        break
    fi
    sleep 1
done

# Pre-load model into VRAM using API (more reliable than interactive mode)
echo "Loading model into VRAM..."
curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen3-30b-aider:latest",
  "prompt": "hi",
  "stream": false,
  "options": {"num_predict": 1}
}' >/dev/null 2>&1
echo "Model loaded."
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

    # Don't pre-load all source files - causes context overflow
    # Let aider discover files via repo map instead
    aider \
        AIDER_INSTRUCTIONS.md \
        Cargo.toml \
        --no-stream \
        --yes \
        --map-tokens 1024 \
        --max-chat-history-tokens 2048 \
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

        # Kill all ollama processes and wait for them to die
        pkill -9 -f "ollama" 2>/dev/null
        sleep 2

        # Verify they're dead
        while pgrep -f "ollama" >/dev/null 2>&1; do
            echo "Waiting for ollama processes to terminate..."
            pkill -9 -f "ollama" 2>/dev/null
            sleep 1
        done

        # Start fresh instance
        ollama serve &>/dev/null &
        sleep 3

        # Wait for ollama to be ready
        echo "Waiting for ollama..."
        for i in {1..30}; do
            if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
                echo "Ollama is ready."
                break
            fi
            sleep 1
        done

        # Load the model using API
        echo "Loading model into VRAM..."
        curl -s http://localhost:11434/api/generate -d '{
          "model": "qwen3-30b-aider:latest",
          "prompt": "hi",
          "stream": false,
          "options": {"num_predict": 1}
        }' >/dev/null 2>&1
        echo "Model loaded."
    fi

    COMMITS_AFTER=$(git rev-list --count HEAD 2>/dev/null || echo "0")
    NEW_COMMITS=$((COMMITS_AFTER - COMMITS))

    # HALLUCINATION DETECTION: Check for uncommitted changes
    DIRTY_FILES=$(git status --porcelain 2>/dev/null | wc -l)

    echo ""
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│ Session $SESSION complete (exit: $EXIT_CODE)"
    echo "│ New commits: $NEW_COMMITS | Uncommitted: $DIRTY_FILES"
    echo "└─────────────────────────────────────────────────────────────┘"

    # If there are dirty files, check if they compile
    if [ "$DIRTY_FILES" -gt 0 ]; then
        echo ""
        echo "Detected uncommitted changes, testing if they compile..."
        if RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
            echo "✓ Build passes! Auto-committing aider's work..."
            git add -A
            git commit -m "Auto-commit: aider changes that compile"
            NEW_COMMITS=1
        else
            echo "✗ Build FAILS - reverting hallucinated/broken code..."
            git checkout -- .
            git clean -fd 2>/dev/null
            echo "Reverted to last working state."
        fi
    fi

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
            echo "════════════════════════════════════════════════════════════"
            echo "Calling Claude Code for help..."
            echo "════════════════════════════════════════════════════════════"
            BUILD_OUTPUT=$(RUSTFLAGS="-D warnings" cargo build --release 2>&1 | tail -50)

            # Snapshot file state before Claude runs
            FILES_BEFORE=$(find src -name "*.rs" -exec md5sum {} \; 2>/dev/null | sort)
            INSTRUCTIONS_BEFORE=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)

            # Run Claude non-interactively with --print and skip permissions
            # --dangerously-skip-permissions allows file edits and bash without prompts
            timeout 300 claude --print --dangerously-skip-permissions "
The local AI (aider with qwen3-30b) is stuck on this RustOS project.

Current task from AIDER_INSTRUCTIONS.md:
$NEXT_TASKS

Last build output:
$BUILD_OUTPUT

Please:
1. Read src/lib.rs and any relevant source files
2. Create or fix the files needed for the current task
3. Run: RUSTFLAGS=\"-D warnings\" cargo build --release
4. Fix any errors until build passes with zero warnings
5. Update AIDER_INSTRUCTIONS.md to mark [x] the completed task
6. Commit and push the changes

Work autonomously until the task is complete.
"
            CLAUDE_EXIT=$?

            # Verify Claude actually made changes (anti-hallucination check)
            FILES_AFTER=$(find src -name "*.rs" -exec md5sum {} \; 2>/dev/null | sort)
            INSTRUCTIONS_AFTER=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)
            DIRTY_AFTER=$(git status --porcelain 2>/dev/null | wc -l)
            COMMITS_AFTER_CLAUDE=$(git rev-list --count HEAD 2>/dev/null || echo "0")

            if [ "$FILES_BEFORE" = "$FILES_AFTER" ] && [ "$INSTRUCTIONS_BEFORE" = "$INSTRUCTIONS_AFTER" ] && [ "$DIRTY_AFTER" -eq 0 ] && [ "$COMMITS_AFTER_CLAUDE" -eq "$COMMITS_AFTER" ]; then
                echo ""
                echo "⚠ Claude claimed to work but made NO actual changes!"
                echo "  Files unchanged, no commits, no dirty files."
                echo "  This was likely a hallucination. Continuing..."
                # Don't reset stuck count - let it try again or escalate
            else
                echo ""
                echo "✓ Claude made actual changes."
                # Check if changes compile
                if [ "$DIRTY_AFTER" -gt 0 ]; then
                    echo "Testing uncommitted changes..."
                    if RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
                        echo "✓ Build passes! Auto-committing Claude's work..."
                        git add -A
                        git commit -m "Auto-commit: Claude Code changes that compile"
                        git push origin master
                    else
                        echo "✗ Build FAILS - reverting Claude's broken code..."
                        git checkout -- .
                        git clean -fd 2>/dev/null
                    fi
                fi
                STUCK_COUNT=0
            fi
        fi
    fi

    # Periodic sanity check every 5 sessions (uses haiku to keep costs low)
    if [ $((SESSION % 5)) -eq 0 ] && [ $SESSION -gt 0 ]; then
        echo ""
        echo "Running periodic sanity check (session $SESSION)..."
        BUILD_CHECK=$(RUSTFLAGS="-D warnings" cargo build --release 2>&1 | tail -50)
        if echo "$BUILD_CHECK" | grep -q "error"; then
            echo "⚠ Sanity check found build errors - calling Claude haiku to fix..."

            # Run haiku non-interactively
            timeout 120 claude --print --dangerously-skip-permissions --model haiku "
Quick sanity check on RustOS project. Build is failing (last 50 lines):

$BUILD_CHECK

Please fix any issues and ensure build passes. Be brief.
"
            # Verify and commit haiku's changes
            DIRTY_HAIKU=$(git status --porcelain 2>/dev/null | wc -l)
            if [ "$DIRTY_HAIKU" -gt 0 ]; then
                if RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
                    echo "✓ Haiku fixed the build!"
                    git add -A
                    git commit -m "Auto-commit: Claude haiku build fix"
                    git push origin master
                else
                    echo "✗ Haiku's fix didn't work - reverting..."
                    git checkout -- .
                    git clean -fd 2>/dev/null
                fi
            fi
        else
            echo "✓ Sanity check passed - build OK"
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

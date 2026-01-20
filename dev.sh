#!/bin/bash
# Continuous development script for RustOS

cd "$(dirname "$0")"

# === DEBUG / LOGGING ===
DEBUG=1
LOG_FILE="dev.log"
START_TIME=$(date +%s)

# Stats counters
STAT_SESSIONS=0
STAT_OLLAMA_RESTARTS=0
STAT_MODEL_LOAD_FAILURES=0
STAT_AIDER_TIMEOUTS=0
STAT_AIDER_CRASHES=0
STAT_HEALTH_CHECK_FAILURES=0
STAT_BUILD_FAILURES=0
STAT_HALLUCINATIONS=0
STAT_MISSING_FILES=0
STAT_MISPLACED_FILES=0
STAT_STUCK_EVENTS=0
STAT_CLAUDE_CALLS=0
STAT_CLAUDE_HALLUCINATIONS=0
STAT_PLANNING_SESSIONS=0
STAT_REVERTS=0
STAT_COMMITS=0

# Configuration
PLANNING_INTERVAL=20  # Run planning session every N sessions

# Log function - writes to file and optionally to stdout
log() {
    local level="$1"
    shift
    local msg="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    local entry="[$timestamp] [$level] $msg"

    if [ "$DEBUG" -eq 1 ]; then
        echo "$entry" >> "$LOG_FILE"
    fi
}

# Print summary stats
print_stats() {
    local end_time=$(date +%s)
    local duration=$((end_time - START_TIME))
    local hours=$((duration / 3600))
    local minutes=$(((duration % 3600) / 60))

    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "                    SESSION SUMMARY"
    echo "════════════════════════════════════════════════════════════"
    echo "Runtime: ${hours}h ${minutes}m"
    echo "Sessions completed: $STAT_SESSIONS"
    echo ""
    echo "Issues encountered:"
    echo "  Ollama restarts:        $STAT_OLLAMA_RESTARTS"
    echo "  Model load failures:    $STAT_MODEL_LOAD_FAILURES"
    echo "  Health check failures:  $STAT_HEALTH_CHECK_FAILURES"
    echo "  Aider timeouts:         $STAT_AIDER_TIMEOUTS"
    echo "  Aider crashes:          $STAT_AIDER_CRASHES"
    echo "  Build failures:         $STAT_BUILD_FAILURES"
    echo "  Hallucinations:         $STAT_HALLUCINATIONS"
    echo "  Missing file claims:    $STAT_MISSING_FILES"
    echo "  Misplaced files fixed:  $STAT_MISPLACED_FILES"
    echo "  Code reverts:           $STAT_REVERTS"
    echo "  Stuck events:           $STAT_STUCK_EVENTS"
    echo ""
    echo "Claude usage:"
    echo "  Claude calls:           $STAT_CLAUDE_CALLS"
    echo "  Claude hallucinations:  $STAT_CLAUDE_HALLUCINATIONS"
    echo "  Planning sessions:      $STAT_PLANNING_SESSIONS"
    echo ""
    echo "Progress:"
    echo "  Commits made:           $STAT_COMMITS"
    echo "════════════════════════════════════════════════════════════"

    # Also log the summary
    log "INFO" "=== SESSION SUMMARY ==="
    log "INFO" "Runtime: ${hours}h ${minutes}m, Sessions: $STAT_SESSIONS"
    log "INFO" "Ollama restarts: $STAT_OLLAMA_RESTARTS, Model failures: $STAT_MODEL_LOAD_FAILURES"
    log "INFO" "Aider timeouts: $STAT_AIDER_TIMEOUTS, Aider crashes: $STAT_AIDER_CRASHES"
    log "INFO" "Build failures: $STAT_BUILD_FAILURES, Missing files: $STAT_MISSING_FILES, Reverts: $STAT_REVERTS"
    log "INFO" "Stuck events: $STAT_STUCK_EVENTS, Claude calls: $STAT_CLAUDE_CALLS"
    log "INFO" "Commits: $STAT_COMMITS"
}

# Cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    log "INFO" "Shutdown requested"
    print_stats
    pkill -9 -f "ollama" 2>/dev/null
    exit 0
}
trap cleanup SIGINT SIGTERM

# Initialize log file
echo "" >> "$LOG_FILE"
log "INFO" "========================================"
log "INFO" "dev.sh started"
log "INFO" "========================================"

# Use only RTX 5080 (16GB) - GPU 1
export CUDA_DEVICE_ORDER=PCI_BUS_ID
export CUDA_VISIBLE_DEVICES=1
export GPU_DEVICE_ORDINAL=1
export OLLAMA_FLASH_ATTENTION=1
export OLLAMA_KV_CACHE_TYPE=q8_0
export OLLAMA_KEEP_ALIVE=-1
export OLLAMA_GPU_LAYERS=999  # Let ollama auto-detect - 8B fits easily in 16GB
# RTX 5080 16GB can handle 64k context - using all that VRAM!

echo "Starting RustOS continuous development..."
echo "Press Ctrl+C to stop"
echo "Log file: $LOG_FILE"
echo ""

# Kill any existing ollama/aider processes and reap zombies
echo "Cleaning up any existing processes..."
pkill -9 -f "ollama" 2>/dev/null
pkill -9 -f "bin/aider" 2>/dev/null

# Reap any zombie children from this shell
wait 2>/dev/null

# Wait for live (non-zombie) processes to die
for i in {1..10}; do
    # Count non-zombie ollama processes
    LIVE_OLLAMA=$(pgrep -f "ollama" | xargs -r ps -o pid=,state= -p 2>/dev/null | grep -v " Z" | wc -l)
    if [ "$LIVE_OLLAMA" -eq 0 ]; then
        break
    fi
    echo "Waiting for $LIVE_OLLAMA ollama process(es) to terminate..."
    pkill -9 -f "ollama" 2>/dev/null
    sleep 1
done

# Orphan any remaining zombies by killing their parent shells (stopped dev.sh instances)
ZOMBIE_COUNT=$(ps aux | grep -E 'ollama|aider' | grep ' Z ' | wc -l)
if [ "$ZOMBIE_COUNT" -gt 0 ]; then
    echo "Cleaning up $ZOMBIE_COUNT zombie process(es)..."
    # Kill any stopped dev.sh processes (state T) which are holding zombies
    ps aux | grep 'dev.sh' | grep ' T ' | awk '{print $2}' | xargs -r kill -9 2>/dev/null
    sleep 1
fi

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

# Function to load model with timeout and retries
load_model() {
    local max_attempts=3
    local timeout_secs=120

    for attempt in $(seq 1 $max_attempts); do
        echo "Loading model into VRAM (attempt $attempt/$max_attempts)..."
        log "INFO" "Model load attempt $attempt/$max_attempts"

        # Use timeout on curl to prevent hanging
        if timeout $timeout_secs curl -s http://localhost:11434/api/generate -d '{
          "model": "llama3.1:64k",
          "prompt": "hi",
          "stream": false,
          "options": {"num_predict": 1}
        }' >/dev/null 2>&1; then
            echo "Model loaded successfully."
            log "INFO" "Model loaded successfully"

            # Warmup: pre-allocate KV cache with realistic prompt
            echo "Warming up model..."
            log "INFO" "Warming up model KV cache"
            timeout 120 curl -s http://localhost:11434/api/generate -d '{
              "model": "llama3.1:64k",
              "prompt": "You are a coding assistant. Implement the next feature.",
              "stream": false,
              "options": {"num_predict": 20}
            }' >/dev/null 2>&1

            if [ $? -eq 0 ]; then
                echo "✓ Model warmed up and ready"
                log "INFO" "Model warmup completed successfully"
            else
                echo "⚠ Warmup timed out, but model is loaded"
                log "WARN" "Model warmup timed out but continuing"
            fi

            return 0
        else
            echo "Model load failed or timed out."
            log "WARN" "Model load failed/timed out (attempt $attempt)"
            STAT_MODEL_LOAD_FAILURES=$((STAT_MODEL_LOAD_FAILURES + 1))
            if [ $attempt -lt $max_attempts ]; then
                echo "Restarting ollama and retrying..."
                log "INFO" "Restarting ollama for model load retry"
                STAT_OLLAMA_RESTARTS=$((STAT_OLLAMA_RESTARTS + 1))
                pkill -9 -f "ollama" 2>/dev/null
                wait 2>/dev/null
                sleep 2
                ollama serve &>/dev/null &
                sleep 3
                # Wait for ollama API
                for i in {1..30}; do
                    if curl -s --max-time 5 http://localhost:11434/api/tags >/dev/null 2>&1; then
                        break
                    fi
                    sleep 1
                done
            fi
        fi
    done

    echo "ERROR: Failed to load model after $max_attempts attempts"
    log "ERROR" "Failed to load model after $max_attempts attempts"
    return 1
}

# Function to check if ollama is responsive
check_ollama_health() {
    curl -s --max-time 5 http://localhost:11434/api/tags >/dev/null 2>&1
}

# Load the model
if ! load_model; then
    echo "Could not load model. Exiting."
    log "ERROR" "Initial model load failed, exiting"
    exit 1
fi
echo ""

SESSION=0
STUCK_COUNT=0
LAST_TASK_HASH=""
SAME_TASK_COUNT=0

while true; do
    SESSION=$((SESSION + 1))
    STAT_SESSIONS=$((STAT_SESSIONS + 1))
    COMMITS=$(git rev-list --count HEAD 2>/dev/null || echo "0")
    DONE=$(grep -c "\[x\]" AIDER_INSTRUCTIONS.md 2>/dev/null || echo "0")
    TODO=$(grep -c "\[ \]" AIDER_INSTRUCTIONS.md 2>/dev/null || echo "0")

    # Get next 3 unchecked items
    NEXT_TASKS=$(grep -m3 "\[ \]" AIDER_INSTRUCTIONS.md | sed 's/- \[ \] /  - /')
    NEXT_TASK_ONELINE=$(echo "$NEXT_TASKS" | head -1 | sed 's/^[[:space:]]*//')

    # Track if we're stuck on the same task
    CURRENT_TASK_HASH=$(echo "$NEXT_TASK_ONELINE" | md5sum | cut -d' ' -f1)
    if [ "$CURRENT_TASK_HASH" = "$LAST_TASK_HASH" ]; then
        SAME_TASK_COUNT=$((SAME_TASK_COUNT + 1))
    else
        SAME_TASK_COUNT=0
        LAST_TASK_HASH="$CURRENT_TASK_HASH"
    fi

    log "INFO" "--- Session $SESSION started ---"
    log "INFO" "Task: $NEXT_TASK_ONELINE"
    log "INFO" "Progress: Done=$DONE, Todo=$TODO, Same task count=$SAME_TASK_COUNT"

    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║ Session: $SESSION | $(date '+%Y-%m-%d %H:%M:%S')"
    echo "║ Commits: $COMMITS | Done: $DONE | Todo: $TODO"
    if [ $SAME_TASK_COUNT -gt 1 ]; then
        echo "║ ⚠ Same task attempt: $SAME_TASK_COUNT (will escalate at 3)"
    fi
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""

    # CHECK BUILD STATUS - tell aider about errors if any
    echo "Checking build status..."
    BUILD_PRE_CHECK=$(RUSTFLAGS="-D warnings" cargo build --release 2>&1)
    # Show more errors with larger context available (20 lines)
    BUILD_ERRORS=$(echo "$BUILD_PRE_CHECK" | grep -E "^error|^warning" | head -20)

    if echo "$BUILD_PRE_CHECK" | grep -q "^error"; then
        echo "⚠ Build has errors - telling aider to fix them first..."
        log "WARN" "Build broken at session start"
        STAT_BUILD_FAILURES=$((STAT_BUILD_FAILURES + 1))
        BUILD_STATUS_MSG="URGENT: Build is broken. Fix these errors FIRST:

\`\`\`
$BUILD_ERRORS
\`\`\`

After fixing: RUSTFLAGS=\"-D warnings\" cargo build --release
"
    else
        echo "✓ Build OK"
        log "INFO" "Build OK at session start"
        BUILD_STATUS_MSG=""
    fi

    # Pre-flight check: ensure ollama is healthy before starting aider
    if ! check_ollama_health; then
        echo "⚠ Ollama not responding, restarting..."
        log "WARN" "Health check failed, restarting ollama"
        STAT_HEALTH_CHECK_FAILURES=$((STAT_HEALTH_CHECK_FAILURES + 1))
        STAT_OLLAMA_RESTARTS=$((STAT_OLLAMA_RESTARTS + 1))
        pkill -9 -f "ollama" 2>/dev/null
        wait 2>/dev/null
        sleep 2
        ollama serve &>/dev/null &
        sleep 3
        if ! load_model; then
            echo "Failed to restart ollama, skipping this session..."
            log "ERROR" "Failed to restart ollama, skipping session"
            sleep 5
            continue
        fi
    fi

    # 8B model on RTX 5080 - can handle massive 64k context
    # Context budget: 64k total
    #   - 16k map tokens (repo structure + file summaries)
    #   - 16k chat history (conversation memory)
    #   - ~31k available for actual file content
    # No file size limits - can read entire large files (500+ lines)
    log "INFO" "Starting aider session"
    timeout 900 aider \
        AIDER_INSTRUCTIONS.md \
        --model ollama/llama3.1:64k \
        --no-stream \
        --yes \
        --auto-commits \
        --map-tokens 16384 \
        --max-chat-history-tokens 16384 \
        --env-file /dev/null \
        --encoding utf-8 \
        --show-model-warnings \
        --message "
$BUILD_STATUS_MSG
Work on: $NEXT_TASKS

After EVERY change: RUSTFLAGS=\"-D warnings\" cargo build --release
Mark [x] in AIDER_INSTRUCTIONS.md when task is complete and build passes.

IMPORTANT: This is attempt #$((SAME_TASK_COUNT + 1)). If you can't complete it, explain why.
"

    EXIT_CODE=$?
    log "INFO" "Aider exited with code $EXIT_CODE"

    # If aider timed out (exit 124) or crashed, restart ollama to clear VRAM
    if [ $EXIT_CODE -eq 124 ]; then
        echo ""
        echo "⚠ Aider session timed out (15 min limit). Likely ollama hung."
        echo "Killing aider and restarting ollama..."
        log "WARN" "Aider timed out (15 min limit)"
        STAT_AIDER_TIMEOUTS=$((STAT_AIDER_TIMEOUTS + 1))
        pkill -9 -f "bin/aider" 2>/dev/null
    fi

    if [ $EXIT_CODE -ne 0 ]; then
        echo ""
        echo "Aider exited with code $EXIT_CODE. Restarting ollama..."

        if [ $EXIT_CODE -ne 124 ]; then
            log "WARN" "Aider crashed with exit code $EXIT_CODE"
            STAT_AIDER_CRASHES=$((STAT_AIDER_CRASHES + 1))
        fi

        STAT_OLLAMA_RESTARTS=$((STAT_OLLAMA_RESTARTS + 1))

        # Explicitly unload model from VRAM before killing ollama
        echo "Unloading model from VRAM..."
        curl -s -X DELETE http://localhost:11434/api/generate \
            -d '{"model":"llama3.1:64k","keep_alive":0}' \
            --max-time 5 2>/dev/null || true
        sleep 2

        # Kill all ollama processes and reap zombies
        pkill -9 -f "ollama" 2>/dev/null
        wait 2>/dev/null
        sleep 2

        # Wait for live processes to die (max 10 seconds)
        for i in {1..10}; do
            LIVE_OLLAMA=$(pgrep -f "ollama" | xargs -r ps -o pid=,state= -p 2>/dev/null | grep -v " Z" | wc -l)
            [ "$LIVE_OLLAMA" -eq 0 ] && break
            echo "Waiting for $LIVE_OLLAMA ollama process(es)..."
            pkill -9 -f "ollama" 2>/dev/null
            sleep 1
        done

        # Force VRAM cleanup - wait for GPU to fully release memory
        echo "Waiting for VRAM cleanup..."
        sleep 8

        # Start fresh instance and load model using the robust function
        ollama serve &>/dev/null &
        sleep 5
        load_model
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
        log "INFO" "Testing $DIRTY_FILES uncommitted files"

        # AUTO-FIX: Move misplaced .rs files from root to src/ (common aider mistake)
        # If lib.rs has 'mod foo;' and foo.rs exists in root but not in src/, move it
        if [ -f src/lib.rs ]; then
            for mod_name in $(grep -oP '(?<=^mod )\w+(?=;)' src/lib.rs 2>/dev/null); do
                if [ -f "${mod_name}.rs" ] && [ ! -f "src/${mod_name}.rs" ]; then
                    echo "⚠ Auto-fixing: Moving ${mod_name}.rs to src/${mod_name}.rs"
                    log "WARN" "Auto-fixing misplaced file: ${mod_name}.rs -> src/${mod_name}.rs"
                    mv "${mod_name}.rs" "src/${mod_name}.rs"
                    STAT_MISPLACED_FILES=$((STAT_MISPLACED_FILES + 1))
                fi
            done
        fi

        # PRE-BUILD CHECK: Detect missing module files (common hallucination pattern)
        # Check if lib.rs has mod statements for files that don't exist
        MISSING_MODS=""
        if [ -f src/lib.rs ]; then
            for mod_name in $(grep -oP '(?<=^mod )\w+(?=;)' src/lib.rs 2>/dev/null); do
                if [ ! -f "src/${mod_name}.rs" ] && [ ! -d "src/${mod_name}" ]; then
                    MISSING_MODS="$MISSING_MODS $mod_name"
                fi
            done
        fi

        if [ -n "$MISSING_MODS" ]; then
            echo "✗ HALLUCINATION DETECTED: mod statement(s) without files:$MISSING_MODS"
            log "WARN" "Missing module files detected:$MISSING_MODS (aider claimed to create but didn't)"
            STAT_MISSING_FILES=$((STAT_MISSING_FILES + 1))
            STAT_HALLUCINATIONS=$((STAT_HALLUCINATIONS + 1))
            STAT_REVERTS=$((STAT_REVERTS + 1))
            git checkout -- .
            git clean -fd 2>/dev/null
            echo "Reverted to last working state."
        elif RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
            echo "✓ Build passes! Auto-committing aider's work..."
            log "INFO" "Uncommitted changes compile, auto-committing"
            git add -A
            git commit -m "Auto-commit: aider changes that compile"
            NEW_COMMITS=1
            STAT_COMMITS=$((STAT_COMMITS + 1))
        else
            echo "✗ Build FAILS - reverting hallucinated/broken code..."
            log "WARN" "Uncommitted changes don't compile, reverting (hallucination)"
            STAT_HALLUCINATIONS=$((STAT_HALLUCINATIONS + 1))
            STAT_REVERTS=$((STAT_REVERTS + 1))
            git checkout -- .
            git clean -fd 2>/dev/null
            echo "Reverted to last working state."
        fi
    fi

    if [ $NEW_COMMITS -gt 0 ]; then
        echo ""
        echo "Recent commits:"
        git log --oneline -n $NEW_COMMITS

        # CRITICAL: Verify the committed code actually compiles before pushing
        # This catches cases where aider commits broken code (e.g., mod statement without file)
        echo ""
        echo "Verifying committed code compiles before pushing..."
        if ! RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
            echo "✗ Committed code FAILS to build - reverting commits!"
            log "ERROR" "Committed code doesn't compile, reverting $NEW_COMMITS commit(s)"
            STAT_BUILD_FAILURES=$((STAT_BUILD_FAILURES + 1))
            STAT_REVERTS=$((STAT_REVERTS + 1))
            git reset --hard HEAD~$NEW_COMMITS
            echo "Reverted to pre-session state."
        else
            echo "✓ Build verified"
            echo ""
            echo "Pushing to origin..."
            git push origin master
            log "INFO" "Pushed $NEW_COMMITS commit(s)"
        fi
        STUCK_COUNT=0
    else
        STUCK_COUNT=$((STUCK_COUNT + 1))
        STAT_STUCK_EVENTS=$((STAT_STUCK_EVENTS + 1))
        echo "No progress made (stuck count: $STUCK_COUNT)"
        log "WARN" "No progress, stuck count: $STUCK_COUNT"
    fi

    # Escalate if stuck on same task for too long OR no commits
    # REDUCED from 8 to 3 - don't let aider spin its wheels
    if [ $STUCK_COUNT -ge 2 ] || [ $SAME_TASK_COUNT -ge 3 ]; then
        if [ $SAME_TASK_COUNT -ge 3 ]; then
            echo ""
            echo "════════════════════════════════════════════════════════════"
            echo "⚠ TASK LOOP DETECTED: Same task for $SAME_TASK_COUNT sessions!"
            echo "Local AI is making changes but not completing the task."
            echo "Escalating to Claude Code..."
            echo "════════════════════════════════════════════════════════════"
            log "WARN" "Task loop: $SAME_TASK_COUNT sessions on same task"
        elif [ $STUCK_COUNT -ge 2 ]; then
            echo ""
            echo "════════════════════════════════════════════════════════════"
            echo "Calling Claude Code for help..."
            echo "════════════════════════════════════════════════════════════"
        fi

        log "INFO" "Escalating to Claude Code"
        STAT_CLAUDE_CALLS=$((STAT_CLAUDE_CALLS + 1))
        # Show reasonable amount of errors for debugging
        BUILD_OUTPUT=$(RUSTFLAGS="-D warnings" cargo build --release 2>&1 | grep -E "^error|^warning" | head -20)

        # Snapshot file state before Claude runs
        FILES_BEFORE=$(find src -name "*.rs" -exec md5sum {} \; 2>/dev/null | sort)
        INSTRUCTIONS_BEFORE=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)

        # Build extended context if stuck on same task
        CONTEXT_MSG="The local AI (aider with llama 3.1 8b) is stuck on this RustOS project."
        if [ $SAME_TASK_COUNT -ge 3 ]; then
            CONTEXT_MSG="⚠ TASK LOOP: The local AI has attempted this same task for $SAME_TASK_COUNT sessions, making changes that compile but never marking it complete. This task may be:
1. Too vague or ambiguous
2. Already complete (but needs checkbox marking)
3. Impossible with current codebase
4. Misunderstood by the local AI

Please investigate whether the task is actually done, needs clarification, or should be skipped."
        fi

        # Run Claude non-interactively with --print and skip permissions
        # --dangerously-skip-permissions allows file edits and bash without prompts
        timeout 300 claude --print --dangerously-skip-permissions "
$CONTEXT_MSG

Current task from AIDER_INSTRUCTIONS.md (attempted $SAME_TASK_COUNT times):
$NEXT_TASKS

Last build output:
$BUILD_OUTPUT

Please:
1. Read src/lib.rs and any relevant source files
2. Determine if the task is actually complete (if so, just mark [x])
3. If not complete, create or fix the files needed
4. Run: RUSTFLAGS=\"-D warnings\" cargo build --release
5. Fix any errors until build passes with zero warnings
6. Update AIDER_INSTRUCTIONS.md to mark [x] the completed task
7. If the task is impossible/unclear, mark [x] with a note and move on
8. Commit and push the changes

Work autonomously until the task is resolved.
"
        CLAUDE_EXIT=$?
        log "INFO" "Claude exited with code $CLAUDE_EXIT"

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
            log "WARN" "Claude hallucination - no actual changes made"
            STAT_CLAUDE_HALLUCINATIONS=$((STAT_CLAUDE_HALLUCINATIONS + 1))
            # Don't reset stuck count - let it try again or escalate
        else
            echo ""
            echo "✓ Claude made actual changes."
            log "INFO" "Claude made actual changes"
            # Check if changes compile
            if [ "$DIRTY_AFTER" -gt 0 ]; then
                echo "Testing uncommitted changes..."
                if RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
                    echo "✓ Build passes! Auto-committing Claude's work..."
                    log "INFO" "Claude changes compile, auto-committing"
                    git add -A
                    git commit -m "Auto-commit: Claude Code changes that compile"
                    # Verify commit before pushing
                    if RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
                        git push origin master
                        STAT_COMMITS=$((STAT_COMMITS + 1))
                    else
                        echo "✗ Committed code fails - reverting"
                        log "ERROR" "Claude commit fails build, reverting"
                        STAT_REVERTS=$((STAT_REVERTS + 1))
                        git reset --hard HEAD~1
                    fi
                else
                    echo "✗ Build FAILS - reverting Claude's broken code..."
                    log "WARN" "Claude changes don't compile, reverting"
                    STAT_REVERTS=$((STAT_REVERTS + 1))
                    git checkout -- .
                    git clean -fd 2>/dev/null
                fi
            fi
            STUCK_COUNT=0
            # Reset same-task counter if Claude intervened successfully
            SAME_TASK_COUNT=0
        fi
    fi

    # Periodic sanity check every 5 sessions (uses haiku to keep costs low)
    if [ $((SESSION % 5)) -eq 0 ] && [ $SESSION -gt 0 ]; then
        echo ""
        echo "Running periodic sanity check (session $SESSION)..."
        log "INFO" "Periodic sanity check at session $SESSION"
        BUILD_CHECK_FULL=$(RUSTFLAGS="-D warnings" cargo build --release 2>&1)
        BUILD_CHECK=$(echo "$BUILD_CHECK_FULL" | grep -E "^error|^warning" | head -15)
        if echo "$BUILD_CHECK_FULL" | grep -q "^error"; then
            echo "⚠ Sanity check found build errors - calling Claude haiku to fix..."
            log "WARN" "Sanity check failed, calling haiku"
            STAT_CLAUDE_CALLS=$((STAT_CLAUDE_CALLS + 1))

            # Run haiku non-interactively
            timeout 120 claude --print --dangerously-skip-permissions --model haiku "
Quick sanity check on RustOS project. Build errors:

$BUILD_CHECK

Please fix any issues and ensure build passes. Be brief.
"
            # Verify and commit haiku's changes
            DIRTY_HAIKU=$(git status --porcelain 2>/dev/null | wc -l)
            if [ "$DIRTY_HAIKU" -gt 0 ]; then
                if RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
                    echo "✓ Haiku fixed the build!"
                    log "INFO" "Haiku fixed the build"
                    git add -A
                    git commit -m "Auto-commit: Claude haiku build fix"
                    # Verify commit before pushing
                    if RUSTFLAGS="-D warnings" cargo build --release 2>&1; then
                        git push origin master
                        STAT_COMMITS=$((STAT_COMMITS + 1))
                    else
                        echo "✗ Committed code fails - reverting"
                        log "ERROR" "Haiku commit fails build, reverting"
                        STAT_REVERTS=$((STAT_REVERTS + 1))
                        git reset --hard HEAD~1
                    fi
                else
                    echo "✗ Haiku's fix didn't work - reverting..."
                    log "WARN" "Haiku fix failed, reverting"
                    STAT_REVERTS=$((STAT_REVERTS + 1))
                    git checkout -- .
                    git clean -fd 2>/dev/null
                fi
            fi
        else
            echo "✓ Sanity check passed - build OK"
            log "INFO" "Sanity check passed"
        fi
    fi

    # Strategic planning session every N sessions (uses Opus for high-level thinking)
    if [ $((SESSION % PLANNING_INTERVAL)) -eq 0 ] && [ $SESSION -gt 0 ]; then
        echo ""
        echo "╔════════════════════════════════════════════════════════════╗"
        echo "║           STRATEGIC PLANNING SESSION                       ║"
        echo "╚════════════════════════════════════════════════════════════╝"
        log "INFO" "Starting planning session at session $SESSION"
        STAT_PLANNING_SESSIONS=$((STAT_PLANNING_SESSIONS + 1))
        STAT_CLAUDE_CALLS=$((STAT_CLAUDE_CALLS + 1))

        # Gather context for planning
        COMPLETED_TASKS=$(grep "\[x\]" AIDER_INSTRUCTIONS.md | tail -10)
        REMAINING_TASKS=$(grep "\[ \]" AIDER_INSTRUCTIONS.md)
        RECENT_COMMITS=$(git log --oneline -10 2>/dev/null)
        CODE_STRUCTURE=$(find src -name "*.rs" -type f 2>/dev/null | head -20)

        # Check for stuck loop
        STUCK_WARNING=""
        if [ $SAME_TASK_COUNT -ge 3 ]; then
            STUCK_WARNING="⚠ STUCK LOOP DETECTED: The local AI has been working on the same task for $SAME_TASK_COUNT sessions without completing it:
Task: $NEXT_TASK_ONELINE

CRITICAL: You MUST resolve this stuck loop. Either:
1. Mark the task [x] as complete if it's already done (check the code!)
2. Rewrite the task to be clearer and more specific
3. Mark [x] and skip it if it's impossible/unclear
4. Break it into smaller, achievable subtasks"
        fi

        # Snapshot before planning
        INSTRUCTIONS_BEFORE=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)

        timeout 600 claude --print --dangerously-skip-permissions "
You are the visionary architect and strategic planner for RustOS - a hobby OS kernel written in Rust.

SESSION STATS:
- Sessions completed: $SESSION
- Tasks done: $DONE
- Tasks remaining: $TODO
- Same task attempts: $SAME_TASK_COUNT

$STUCK_WARNING

RECENT PROGRESS (last 10 commits):
$RECENT_COMMITS

RECENTLY COMPLETED TASKS:
$COMPLETED_TASKS

CURRENT REMAINING TASKS:
$REMAINING_TASKS

CURRENT CODE STRUCTURE:
$CODE_STRUCTURE

YOUR MISSION - EVOLVE THE VISION:

1. RESOLVE STUCK LOOPS (if any): If a task is stuck, investigate the code and either mark it done, clarify it, or skip it

2. CELEBRATE PROGRESS: Review what's been accomplished and how the OS is taking shape

3. EXPAND THE ROADMAP: The roadmap should always be growing. Add new features that would make this a more capable, interesting OS:
   - What's the next logical capability after current tasks?
   - What would make this OS unique or impressive?
   - Consider: filesystems, networking, graphics, shell, userspace programs
   - Think about what features would be fun to implement and demo

4. REFINE PRIORITIES: Reorder tasks so the most impactful/unblocking work comes first

5. ADD DETAIL: For complex upcoming tasks, add implementation hints or break them into subtasks

6. MAINTAIN VISION: Keep a 'Vision' or 'Goals' section at the top describing what this OS is becoming

UPDATE AIDER_INSTRUCTIONS.md:
- FIRST: Resolve any stuck loops (mark done, clarify, or skip)
- Add 3-5 NEW tasks beyond what's currently listed (always be expanding)
- Reorder if needed (dependencies first, then high-impact features)
- Add brief implementation hints for tricky tasks
- Keep checkbox format: - [ ] todo, - [x] done
- Group related tasks under phase headings

PHILOSOPHY:
- Stuck tasks are roadblocks - resolve them immediately
- This OS should keep getting more capable and interesting
- Each planning session should leave the roadmap MORE ambitious, not less
- Balance achievable near-term tasks with exciting longer-term goals
- The local AI (aider) works best with clear, specific tasks

After updating, commit with message: 'docs: planning session - expand roadmap'
"
        PLANNING_EXIT=$?
        log "INFO" "Planning session exited with code $PLANNING_EXIT"

        # Check if roadmap was updated
        INSTRUCTIONS_AFTER=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)
        if [ "$INSTRUCTIONS_BEFORE" != "$INSTRUCTIONS_AFTER" ]; then
            echo "✓ Roadmap updated by planning session"
            log "INFO" "Roadmap was updated by planning session"

            # Commit if not already committed
            DIRTY_PLAN=$(git status --porcelain AIDER_INSTRUCTIONS.md 2>/dev/null | wc -l)
            if [ "$DIRTY_PLAN" -gt 0 ]; then
                git add AIDER_INSTRUCTIONS.md
                git commit -m "docs: planning session - expand roadmap (session $SESSION)"
                git push origin master
                STAT_COMMITS=$((STAT_COMMITS + 1))
                log "INFO" "Committed roadmap expansion"
            fi
            # Reset stuck counter since planning session likely resolved it
            SAME_TASK_COUNT=0
        else
            echo "Roadmap unchanged (planning found no updates needed)"
            log "INFO" "Planning session made no roadmap changes"
        fi
    fi

    # Only restart if there's more work to do
    if [ "$TODO" -eq 0 ]; then
        echo "All tasks complete!"
        log "INFO" "All tasks complete!"
        break
    fi

    echo ""
    sleep 1
done

print_stats

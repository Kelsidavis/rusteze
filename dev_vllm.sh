#!/bin/bash
# Continuous development script for RustOS using vLLM
# vLLM provides better VRAM utilization and larger context support

cd "$(dirname "$0")"

# === DEBUG / LOGGING ===
DEBUG=1
LOG_FILE="dev.log"
START_TIME=$(date +%s)

# Stats counters
STAT_SESSIONS=0
STAT_VLLM_RESTARTS=0
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
VLLM_HOST="http://localhost:8000"
# Qwen2.5-Coder-14B-Instruct: 2x smarter than 7B, fits in ~7-8GB VRAM
# Leaves room for ~24k context (shell.rs is 1559 lines = ~12k tokens)
VLLM_MODEL="Qwen/Qwen2.5-Coder-14B-Instruct"

# Log function
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
    echo "  vLLM restarts:          $STAT_VLLM_RESTARTS"
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

    log "INFO" "=== SESSION SUMMARY ==="
    log "INFO" "Runtime: ${hours}h ${minutes}m, Sessions: $STAT_SESSIONS"
    log "INFO" "vLLM restarts: $STAT_VLLM_RESTARTS, Model failures: $STAT_MODEL_LOAD_FAILURES"
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
    pkill -9 -f "vllm.*api_server" 2>/dev/null
    exit 0
}
trap cleanup SIGINT SIGTERM

# === SETUP ===
echo "════════════════════════════════════════════════════════════"
echo "                    RustOS Dev Loop (vLLM)"
echo "════════════════════════════════════════════════════════════"
log "INFO" "========================================"
log "INFO" "dev.sh started"
log "INFO" "========================================"

# Export vLLM environment variables
export CUDA_DEVICE_ORDER=PCI_BUS_ID
export CUDA_VISIBLE_DEVICES=1  # RTX 5080 only

# Kill any existing vLLM/aider processes
echo "Cleaning up existing processes..."
pkill -9 -f "vllm.*api_server" 2>/dev/null
pkill -9 -f "bin/aider" 2>/dev/null
sleep 2

# Wait for processes to fully terminate
for i in {1..10}; do
    LIVE_VLLM=$(pgrep -f "vllm.*api_server" | wc -l)
    if [ "$LIVE_VLLM" -eq 0 ]; then
        break
    fi
    echo "Waiting for $LIVE_VLLM vLLM process(es) to terminate..."
    pkill -9 -f "vllm.*api_server" 2>/dev/null
    sleep 1
done

# Start vLLM server
echo "Starting vLLM server..."
log "INFO" "Starting vLLM server on port 8000"

nohup python3 -m vllm.entrypoints.openai.api_server \
    --model "$VLLM_MODEL" \
    --host 0.0.0.0 \
    --port 8000 \
    --dtype bfloat16 \
    --max-model-len 24576 \
    --gpu-memory-utilization 0.90 \
    --max-num-seqs 8 \
    --disable-log-requests \
    --trust-remote-code \
    --enforce-eager \
    --disable-custom-all-reduce \
    > vllm.log 2>&1 &

VLLM_PID=$!
log "INFO" "vLLM server started with PID $VLLM_PID"

# Wait for vLLM to be ready
echo "Waiting for vLLM server to start..."
for attempt in {1..60}; do
    if curl -s "$VLLM_HOST/v1/models" > /dev/null 2>&1; then
        echo "✓ vLLM server is ready"
        log "INFO" "vLLM server ready after $attempt seconds"
        break
    fi
    if [ $attempt -eq 60 ]; then
        echo "✗ vLLM server failed to start after 60 seconds"
        log "ERROR" "vLLM server failed to start"
        tail -50 vllm.log
        exit 1
    fi
    sleep 1
done

echo "════════════════════════════════════════════════════════════"
echo ""

# === MAIN LOOP ===
SAME_TASK_COUNT=0
LAST_TASK_HASH=""

while true; do
    STAT_SESSIONS=$((STAT_SESSIONS + 1))
    echo "──────────────────────────────────────────────────────────"
    echo "Session #$STAT_SESSIONS"
    log "INFO" "--- Session $STAT_SESSIONS started ---"

    # Get next unchecked task
    NEXT_TASKS=$(grep -E "^- \[ \]" AIDER_INSTRUCTIONS.md | head -3)
    if [ -z "$NEXT_TASKS" ]; then
        echo "🎉 All tasks completed!"
        log "INFO" "All tasks completed"
        print_stats
        break
    fi

    # Display first task
    FIRST_TASK=$(echo "$NEXT_TASKS" | head -1)
    echo "Next task: $FIRST_TASK"

    # Track if we're stuck on same task
    CURRENT_TASK_HASH=$(echo "$NEXT_TASKS" | md5sum | cut -d' ' -f1)
    if [ "$CURRENT_TASK_HASH" = "$LAST_TASK_HASH" ]; then
        SAME_TASK_COUNT=$((SAME_TASK_COUNT + 1))
        log "WARN" "Same task, count: $SAME_TASK_COUNT"
    else
        SAME_TASK_COUNT=0
        LAST_TASK_HASH="$CURRENT_TASK_HASH"
    fi

    # Count progress
    DONE_COUNT=$(grep -c "^\[x\]" AIDER_INSTRUCTIONS.md || echo 0)
    TODO_COUNT=$(grep -c "^- \[ \]" AIDER_INSTRUCTIONS.md || echo 0)
    log "INFO" "Progress: Done=$DONE_COUNT, Todo=$TODO_COUNT, Same task count=$SAME_TASK_COUNT"

    # Check if build passes before starting
    BUILD_STATUS_MSG=""
    RUSTFLAGS="-D warnings" cargo build --release > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        echo "✓ Build passing"
        log "INFO" "Build OK at session start"
        BUILD_STATUS_MSG="Build status: ✓ PASSING (zero warnings)"
    else
        echo "✗ Build failing - fixing first"
        log "WARN" "Build failing at session start"
        BUILD_STATUS_MSG="Build status: ✗ FAILING - Fix errors first!"
        STAT_BUILD_FAILURES=$((STAT_BUILD_FAILURES + 1))
    fi

    # Escalate to Claude if stuck for 2+ sessions
    if [ $SAME_TASK_COUNT -ge 2 ]; then
        log "INFO" "Escalating to Claude Code"
        echo "⚠ Aider stuck for $SAME_TASK_COUNT sessions, escalating to Claude..."
        STAT_STUCK_EVENTS=$((STAT_STUCK_EVENTS + 1))
        STAT_CLAUDE_CALLS=$((STAT_CLAUDE_CALLS + 1))

        # Snapshot before
        FILES_BEFORE=$(find src -name "*.rs" -exec md5sum {} \; 2>/dev/null | sort)
        INSTRUCTIONS_BEFORE=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)

        # Build extended context if stuck on same task
        CONTEXT_MSG="The local AI (aider with Qwen2.5-Coder 7B on vLLM) is stuck on this RustOS project."
        if [ $SAME_TASK_COUNT -ge 3 ]; then
            CONTEXT_MSG="⚠ TASK LOOP: The local AI has attempted this same task for $SAME_TASK_COUNT sessions, making changes that compile but never marking it complete. This task may be:
1. Too vague or ambiguous
2. Already complete (check if implementation exists)
3. Partially complete (check partial implementation)
4. Blocked by missing dependencies

Please investigate whether the task is actually done, needs clarification, or should be skipped."
        fi

        # Kill any existing Claude processes before starting a new one
        pkill -9 -f "claude --print" 2>/dev/null || true
        sleep 1

        # Run Claude non-interactively with --print and skip permissions
        # --dangerously-skip-permissions allows file edits and bash without prompts
        timeout 300 claude --print --dangerously-skip-permissions "
$CONTEXT_MSG

Current task from AIDER_INSTRUCTIONS.md (attempted $SAME_TASK_COUNT times):
$NEXT_TASKS

Last build output:
$BUILD_STATUS_MSG

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

        # If Claude timed out, ensure cleanup
        if [ $CLAUDE_EXIT -eq 124 ]; then
            echo "⚠ Claude timed out after 5 minutes, killing any remaining processes..."
            log "WARN" "Claude timed out (5 min limit)"
            pkill -9 -f "claude --print" 2>/dev/null || true
            sleep 1
        fi

        # Verify Claude actually made changes (anti-hallucination check)
        FILES_AFTER=$(find src -name "*.rs" -exec md5sum {} \; 2>/dev/null | sort)
        INSTRUCTIONS_AFTER=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)

        if [ "$FILES_BEFORE" = "$FILES_AFTER" ] && [ "$INSTRUCTIONS_BEFORE" = "$INSTRUCTIONS_AFTER" ]; then
            echo "⚠ Claude made no changes - possible hallucination"
            log "WARN" "Claude made no changes"
            STAT_CLAUDE_HALLUCINATIONS=$((STAT_CLAUDE_HALLUCINATIONS + 1))
        else
            log "INFO" "Claude made actual changes"
        fi

        # After Claude intervention, reset stuck counter since we changed approach
        SAME_TASK_COUNT=0
        continue
    fi

    # Snapshot before aider session
    FILES_BEFORE=$(find src -name "*.rs" -exec md5sum {} \; 2>/dev/null | sort)
    INSTRUCTIONS_BEFORE=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)

    # vLLM with Qwen2.5-Coder-14B - uses OpenAI-compatible API
    # Context budget: 24k max (14B model needs more VRAM)
    #   - 768 map tokens (repo structure)
    #   - 1.5k chat history
    #   - ~22k available for files aider explicitly adds
    # 14B is 2x smarter than 7B, should reduce hallucinations
    log "INFO" "Starting aider session"
    timeout 900 aider \
        --model openai/Qwen/Qwen2.5-Coder-14B-Instruct \
        --openai-api-base "$VLLM_HOST/v1" \
        --openai-api-key "dummy" \
        --no-stream \
        --yes \
        --auto-commits \
        --map-tokens 768 \
        --max-chat-history-tokens 1536 \
        --env-file /dev/null \
        --encoding utf-8 \
        --show-model-warnings \
        --message "
$BUILD_STATUS_MSG

Task from AIDER_INSTRUCTIONS.md (attempt #$((SAME_TASK_COUNT + 1))):
$NEXT_TASKS

Instructions:
1. Add ONLY the files you need to /add (don't load all source files)
2. Make your changes
3. Run: RUSTFLAGS=\"-D warnings\" cargo build --release
4. Fix any errors
5. When build passes, edit AIDER_INSTRUCTIONS.md to mark [x] the task
6. If you can't complete it or it's already done, mark [x] with a note

DO NOT load large files like src/shell.rs unless absolutely necessary - it will exceed context.
Start with AIDER_INSTRUCTIONS.md only, add other files as needed.
"

    EXIT_CODE=$?
    log "INFO" "Aider exited with code $EXIT_CODE"

    # Verify aider actually made changes
    FILES_AFTER=$(find src -name "*.rs" -exec md5sum {} \; 2>/dev/null | sort)
    INSTRUCTIONS_AFTER=$(md5sum AIDER_INSTRUCTIONS.md 2>/dev/null)

    if [ "$FILES_BEFORE" = "$FILES_AFTER" ] && [ "$INSTRUCTIONS_BEFORE" = "$INSTRUCTIONS_AFTER" ]; then
        echo "⚠ No progress made"
        log "WARN" "No progress, stuck count: $((SAME_TASK_COUNT + 1))"
        continue
    fi

    log "INFO" "Changes detected"

    # Check if build passes
    RUSTFLAGS="-D warnings" cargo build --release > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        echo "✓ Build passing"
        log "INFO" "Build OK after session"
        STAT_COMMITS=$((STAT_COMMITS + 1))
    else
        echo "✗ Build failing after changes"
        log "ERROR" "Build failed after aider session"
        STAT_BUILD_FAILURES=$((STAT_BUILD_FAILURES + 1))
    fi

    # Small delay between sessions
    sleep 2
done

print_stats

#!/usr/bin/env bash
# CoSyn Live Test Harness
# Reads cosyn-live-test-manifest.csv, runs each prompt through cosyn-cli,
# captures results, and generates a markdown report.
#
# Usage: bash tests/live/run-live-tests.sh [path-to-cosyn-cli]
# Default CLI path: target/release/cosyn-cli.exe

set -euo pipefail

# --- Configuration ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="$SCRIPT_DIR/cosyn-live-test-manifest.csv"
SNAPSHOT_DIR="$SCRIPT_DIR/telemetry-snapshots"
TELEMETRY_FILE="cosyn_telemetry.log"
COST_CEILING="0.50"
# gpt-4o-mini: $0.60 per 1M output tokens
COST_PER_OUTPUT_TOKEN="0.0000006"

CLI="${1:-target/release/cosyn-cli.exe}"

# --- Validate prerequisites ---
if [[ ! -f "$CLI" ]]; then
    echo "ERROR: cosyn-cli not found at $CLI"
    echo "Build first: cargo build --release"
    echo "Or specify path: bash tests/live/run-live-tests.sh /path/to/cosyn-cli.exe"
    exit 1
fi

if [[ -z "${OPENAI_API_KEY:-}" ]]; then
    echo "ERROR: OPENAI_API_KEY not set"
    exit 1
fi

if [[ ! -f "$MANIFEST" ]]; then
    echo "ERROR: Manifest not found at $MANIFEST"
    exit 1
fi

mkdir -p "$SNAPSHOT_DIR"

# --- State ---
DATE=$(date +%Y-%m-%d)
REPORT="$SCRIPT_DIR/cosyn-live-test-report-$DATE.md"
PROMPT_NUM=0
TOTAL=0
TOTAL_PASS=0
TOTAL_BLOCK=0
TOTAL_ERROR=0
TOTAL_MATCH=0
TOTAL_MISMATCH=0
BASELINE_TOTAL=0
BASELINE_PASS=0
BASELINE_BLOCK=0
BASELINE_ERROR=0
BASELINE_MATCH=0
BASELINE_MISMATCH=0
STRESS_TOTAL=0
STRESS_PASS=0
STRESS_BLOCK=0
STRESS_ERROR=0
STRESS_MATCH=0
STRESS_MISMATCH=0
UX_TOTAL=0
UX_PASS=0
UX_BLOCK=0
UX_ERROR=0
UX_MATCH=0
UX_MISMATCH=0
CUMULATIVE_COST="0"
REVISION_TRIGGERS=0

# Storage arrays
declare -a R_NUM R_CAT R_PROMPT R_EXPECTED R_ACTUAL R_MATCH R_ATTEMPTS R_TOKENS R_ELAPSED R_NOTES R_BINDING R_BLOCK_CODE

# --- Helper: parse telemetry ---
# Reads from both the telemetry file snapshot and the stdout snapshot.
# Token/cost data is in stdout; binding/block data is in the telemetry file.
parse_telemetry() {
    local snapshot="$1"
    local stdout_snap="$2"
    local attempts=0
    local tokens=0
    local binding=""
    local block_code=""

    # Attempts and binding from telemetry snapshot
    if [[ -f "$snapshot" ]]; then
        attempts=$(grep -c "llm_call_start" "$snapshot" 2>/dev/null || echo "0")
        local binding_line
        binding_line=$(grep "subject_binding_source" "$snapshot" 2>/dev/null | tail -1 || echo "")
        if [[ -n "$binding_line" ]]; then
            binding=$(echo "$binding_line" | sed 's/.*subject_binding_source = //')
        fi
        local block_line
        block_line=$(grep "FAIL.*BR-" "$snapshot" 2>/dev/null | head -1 || echo "")
        if [[ -n "$block_line" ]]; then
            block_code=$(echo "$block_line" | sed 's/.*\(BR-[A-Z_-]*\).*/\1/')
        fi
    fi

    # Token count from stdout (where USAGE line appears)
    if [[ -f "$stdout_snap" ]]; then
        local token_line
        token_line=$(grep "est_tokens:" "$stdout_snap" 2>/dev/null | tail -1 || echo "")
        if [[ -n "$token_line" ]]; then
            tokens=$(echo "$token_line" | sed 's/.*est_tokens: \([0-9]*\).*/\1/')
        fi
        # Fallback: check telemetry snapshot too
        if [[ "$tokens" -eq 0 ]] && [[ -f "$snapshot" ]]; then
            token_line=$(grep "est_tokens:" "$snapshot" 2>/dev/null | tail -1 || echo "")
            if [[ -n "$token_line" ]]; then
                tokens=$(echo "$token_line" | sed 's/.*est_tokens: \([0-9]*\).*/\1/')
            fi
        fi
    fi

    echo "$attempts|$tokens|$binding|$block_code"
}

# --- Main loop ---
echo "CoSyn Live Test Harness"
echo "CLI: $CLI"
echo "Manifest: $MANIFEST"
echo "Date: $DATE"
echo "Cost ceiling: \$$COST_CEILING"
echo "---"

ABORTED=false

while IFS= read -r line; do
    # Skip comments and empty lines
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// /}" ]] && continue

    # Parse manifest line
    IFS='|' read -r cat prompt expected <<< "$line"
    cat=$(echo "$cat" | xargs)
    prompt=$(echo "$prompt" | xargs)
    expected=$(echo "$expected" | xargs)

    PROMPT_NUM=$((PROMPT_NUM + 1))
    TOTAL=$((TOTAL + 1))

    echo "[$PROMPT_NUM] $cat | $prompt | expected: $expected"

    # Clean telemetry
    rm -f "$TELEMETRY_FILE"

    # Run prompt
    local_start=$(date +%s%N 2>/dev/null || date +%s)
    set +e
    stdout=$("$CLI" "$prompt" 2>/tmp/cosyn_stderr.txt)
    exit_code=$?
    set -e
    local_end=$(date +%s%N 2>/dev/null || date +%s)

    # Compute elapsed (handle platforms without nanoseconds)
    if [[ ${#local_start} -gt 10 ]]; then
        elapsed_ms=$(( (local_end - local_start) / 1000000 ))
    else
        elapsed_ms=$(( (local_end - local_start) * 1000 ))
    fi

    stderr=$(cat /tmp/cosyn_stderr.txt 2>/dev/null || echo "")

    # Snapshot telemetry (file) and stdout separately
    snapshot="$SNAPSHOT_DIR/prompt-$(printf '%02d' $PROMPT_NUM).log"
    stdout_snapshot="$SNAPSHOT_DIR/prompt-$(printf '%02d' $PROMPT_NUM)-stdout.log"
    echo "$stdout" > "$stdout_snapshot"
    if [[ -f "$TELEMETRY_FILE" ]]; then
        cp "$TELEMETRY_FILE" "$snapshot"
    else
        cp "$stdout_snapshot" "$snapshot"
    fi

    # Determine actual result
    if [[ $exit_code -eq 0 ]]; then
        actual="pass"
    elif [[ $exit_code -eq 1 ]]; then
        actual="block"
    else
        actual="error"
    fi

    # Parse telemetry
    telem=$(parse_telemetry "$snapshot" "$stdout_snapshot")
    IFS='|' read -r attempts tokens binding block_code <<< "$telem"
    [[ -z "$attempts" || "$attempts" -eq 0 ]] && attempts=0
    [[ -z "$tokens" ]] && tokens=0

    # Track revision loop triggers
    if [[ "$attempts" -gt 1 ]]; then
        REVISION_TRIGGERS=$((REVISION_TRIGGERS + 1))
    fi

    # Estimate cost (use awk since bc may not be available on Windows Git Bash)
    if [[ "$tokens" -gt 0 ]]; then
        CUMULATIVE_COST=$(awk "BEGIN {printf \"%.8f\", $CUMULATIVE_COST + ($tokens * $COST_PER_OUTPUT_TOKEN)}")
    fi

    # Compute match
    if [[ "$actual" == "$expected" ]]; then
        match="Yes"
        TOTAL_MATCH=$((TOTAL_MATCH + 1))
    else
        match="**NO**"
        TOTAL_MISMATCH=$((TOTAL_MISMATCH + 1))
    fi

    # Build notes
    notes=""
    if [[ "$actual" == "pass" && "$attempts" -gt 0 ]]; then
        notes="$attempts attempt(s), $tokens tokens"
        if [[ -n "$binding" ]]; then
            notes="$notes, $binding"
        fi
    elif [[ "$actual" == "block" ]]; then
        notes="Blocked"
        if [[ -n "$block_code" ]]; then
            notes="$notes: $block_code"
        fi
    elif [[ "$actual" == "error" ]]; then
        notes="ERROR exit code $exit_code"
    fi
    notes="$notes (${elapsed_ms}ms)"

    # Category counters
    case "$cat" in
        baseline)
            BASELINE_TOTAL=$((BASELINE_TOTAL + 1))
            [[ "$actual" == "pass" ]] && BASELINE_PASS=$((BASELINE_PASS + 1))
            [[ "$actual" == "block" ]] && BASELINE_BLOCK=$((BASELINE_BLOCK + 1))
            [[ "$actual" == "error" ]] && BASELINE_ERROR=$((BASELINE_ERROR + 1))
            [[ "$match" == "Yes" ]] && BASELINE_MATCH=$((BASELINE_MATCH + 1)) || BASELINE_MISMATCH=$((BASELINE_MISMATCH + 1))
            ;;
        stress)
            STRESS_TOTAL=$((STRESS_TOTAL + 1))
            [[ "$actual" == "pass" ]] && STRESS_PASS=$((STRESS_PASS + 1))
            [[ "$actual" == "block" ]] && STRESS_BLOCK=$((STRESS_BLOCK + 1))
            [[ "$actual" == "error" ]] && STRESS_ERROR=$((STRESS_ERROR + 1))
            [[ "$match" == "Yes" ]] && STRESS_MATCH=$((STRESS_MATCH + 1)) || STRESS_MISMATCH=$((STRESS_MISMATCH + 1))
            ;;
        ux)
            UX_TOTAL=$((UX_TOTAL + 1))
            [[ "$actual" == "pass" ]] && UX_PASS=$((UX_PASS + 1))
            [[ "$actual" == "block" ]] && UX_BLOCK=$((UX_BLOCK + 1))
            [[ "$actual" == "error" ]] && UX_ERROR=$((UX_ERROR + 1))
            [[ "$match" == "Yes" ]] && UX_MATCH=$((UX_MATCH + 1)) || UX_MISMATCH=$((UX_MISMATCH + 1))
            ;;
    esac

    [[ "$actual" == "pass" ]] && TOTAL_PASS=$((TOTAL_PASS + 1))
    [[ "$actual" == "block" ]] && TOTAL_BLOCK=$((TOTAL_BLOCK + 1))
    [[ "$actual" == "error" ]] && TOTAL_ERROR=$((TOTAL_ERROR + 1))

    # Store results
    R_NUM+=("$PROMPT_NUM")
    R_CAT+=("$cat")
    R_PROMPT+=("$prompt")
    R_EXPECTED+=("$expected")
    R_ACTUAL+=("$actual")
    R_MATCH+=("$match")
    R_ATTEMPTS+=("$attempts")
    R_TOKENS+=("$tokens")
    R_ELAPSED+=("$elapsed_ms")
    R_NOTES+=("$notes")
    R_BINDING+=("$binding")
    R_BLOCK_CODE+=("$block_code")

    echo "  -> actual: $actual | match: $match | $notes"

    # Cost ceiling check
    ceiling_hit=$(awk "BEGIN {print ($CUMULATIVE_COST >= $COST_CEILING) ? 1 : 0}")
    if [[ "$ceiling_hit" == "1" ]]; then
        echo "ABORT: Cost ceiling \$$COST_CEILING reached (cumulative: \$$CUMULATIVE_COST)"
        ABORTED=true
        break
    fi

done < "$MANIFEST"

# --- Compute rates ---
if [[ $BASELINE_TOTAL -gt 0 ]]; then
    FALSE_BLOCK_RATE=$((BASELINE_BLOCK * 100 / BASELINE_TOTAL))
else
    FALSE_BLOCK_RATE=0
fi

STRESS_EXPECTED_BLOCKS=$(grep -c "^stress.*| block" "$MANIFEST" 2>/dev/null || echo "0")
if [[ $STRESS_EXPECTED_BLOCKS -gt 0 ]]; then
    # Count stress prompts expected to block that actually passed
    STRESS_FALSE_PASSES=0
    for i in "${!R_CAT[@]}"; do
        if [[ "${R_CAT[$i]}" == "stress" && "${R_EXPECTED[$i]}" == "block" && "${R_ACTUAL[$i]}" == "pass" ]]; then
            STRESS_FALSE_PASSES=$((STRESS_FALSE_PASSES + 1))
        fi
    done
    FALSE_PASS_RATE=$((STRESS_FALSE_PASSES * 100 / STRESS_EXPECTED_BLOCKS))
else
    FALSE_PASS_RATE=0
    STRESS_FALSE_PASSES=0
fi

TOTAL_PASSED_PROMPTS=$((TOTAL_PASS))
if [[ $TOTAL_PASSED_PROMPTS -gt 0 ]]; then
    REVISION_RATE=$((REVISION_TRIGGERS * 100 / TOTAL_PASSED_PROMPTS))
else
    REVISION_RATE=0
fi

COST_DISPLAY=$(printf "%.6f" "$CUMULATIVE_COST" 2>/dev/null || echo "$CUMULATIVE_COST")

# --- Generate report ---
{
    echo "# CoSyn Live Test Report"
    echo ""
    echo "**Date:** $DATE"
    echo "**Version:** 4.1.0"
    echo "**LLM:** OpenAI gpt-4o-mini"
    echo "**Prompts tested:** $TOTAL"
    if [[ "$ABORTED" == "true" ]]; then
        echo "**Status:** ABORTED — cost ceiling reached"
    else
        echo "**Status:** Complete"
    fi
    echo ""
    echo "---"
    echo ""
    echo "## Computed Metrics"
    echo ""
    echo "| Metric | Value | Threshold |"
    echo "|--------|-------|-----------|"
    echo "| Baseline false-block rate | ${FALSE_BLOCK_RATE}% ($BASELINE_BLOCK/$BASELINE_TOTAL) | Must be 0% for Phase 2 |"
    echo "| Stress false-pass rate | ${FALSE_PASS_RATE}% ($STRESS_FALSE_PASSES/$STRESS_EXPECTED_BLOCKS expected blocks) | Documented |"
    echo "| Revision loop triggers | $REVISION_TRIGGERS/$TOTAL_PASS passed prompts (${REVISION_RATE}%) | Informational |"
    echo "| Estimated API cost | \$$COST_DISPLAY | Must be under \$$COST_CEILING |"
    echo ""
    echo "---"
    echo ""
    echo "## Summary"
    echo ""
    echo "| Category | Total | Pass | Block | Error | Match | Mismatch |"
    echo "|----------|-------|------|-------|-------|-------|----------|"
    echo "| Baseline | $BASELINE_TOTAL | $BASELINE_PASS | $BASELINE_BLOCK | $BASELINE_ERROR | $BASELINE_MATCH | $BASELINE_MISMATCH |"
    echo "| Stress | $STRESS_TOTAL | $STRESS_PASS | $STRESS_BLOCK | $STRESS_ERROR | $STRESS_MATCH | $STRESS_MISMATCH |"
    echo "| UX | $UX_TOTAL | $UX_PASS | $UX_BLOCK | $UX_ERROR | $UX_MATCH | $UX_MISMATCH |"
    echo "| **Total** | **$TOTAL** | **$TOTAL_PASS** | **$TOTAL_BLOCK** | **$TOTAL_ERROR** | **$TOTAL_MATCH** | **$TOTAL_MISMATCH** |"
    echo ""

    # Mismatches section
    HAS_MISMATCH=false
    for i in "${!R_MATCH[@]}"; do
        if [[ "${R_MATCH[$i]}" != "Yes" ]]; then
            HAS_MISMATCH=true
            break
        fi
    done

    if [[ "$HAS_MISMATCH" == "true" ]]; then
        echo "---"
        echo ""
        echo "## Mismatches"
        echo ""
        echo "| # | Category | Prompt | Expected | Actual | Notes |"
        echo "|---|----------|--------|----------|--------|-------|"
        for i in "${!R_MATCH[@]}"; do
            if [[ "${R_MATCH[$i]}" != "Yes" ]]; then
                echo "| ${R_NUM[$i]} | ${R_CAT[$i]} | ${R_PROMPT[$i]} | ${R_EXPECTED[$i]} | ${R_ACTUAL[$i]} | ${R_NOTES[$i]} |"
            fi
        done
        echo ""
    fi

    echo "---"
    echo ""
    echo "## Baseline Results"
    echo ""
    echo "| # | Prompt | Expected | Actual | Match | Attempts | Notes |"
    echo "|---|--------|----------|--------|-------|----------|-------|"
    for i in "${!R_CAT[@]}"; do
        if [[ "${R_CAT[$i]}" == "baseline" ]]; then
            echo "| ${R_NUM[$i]} | ${R_PROMPT[$i]} | ${R_EXPECTED[$i]} | ${R_ACTUAL[$i]} | ${R_MATCH[$i]} | ${R_ATTEMPTS[$i]} | ${R_NOTES[$i]} |"
        fi
    done
    echo ""
    echo "---"
    echo ""
    echo "## Stress Results"
    echo ""
    echo "| # | Prompt | Expected | Actual | Match | Attempts | Notes |"
    echo "|---|--------|----------|--------|-------|----------|-------|"
    for i in "${!R_CAT[@]}"; do
        if [[ "${R_CAT[$i]}" == "stress" ]]; then
            echo "| ${R_NUM[$i]} | ${R_PROMPT[$i]} | ${R_EXPECTED[$i]} | ${R_ACTUAL[$i]} | ${R_MATCH[$i]} | ${R_ATTEMPTS[$i]} | ${R_NOTES[$i]} |"
        fi
    done
    echo ""
    echo "---"
    echo ""
    echo "## UX Results"
    echo ""
    echo "| # | Prompt | Expected | Actual | Match | Attempts | Notes |"
    echo "|---|--------|----------|--------|-------|----------|-------|"
    for i in "${!R_CAT[@]}"; do
        if [[ "${R_CAT[$i]}" == "ux" ]]; then
            echo "| ${R_NUM[$i]} | ${R_PROMPT[$i]} | ${R_EXPECTED[$i]} | ${R_ACTUAL[$i]} | ${R_MATCH[$i]} | ${R_ATTEMPTS[$i]} | ${R_NOTES[$i]} |"
        fi
    done
    echo ""
    echo "---"
    echo ""
    echo "## Telemetry Notes"
    echo ""
    echo "- Revision loop triggers: $REVISION_TRIGGERS"
    echo "- Per-prompt telemetry snapshots saved to: tests/live/telemetry-snapshots/"
    echo ""
    echo "---"
    echo ""
    echo "**End of Report**"

} > "$REPORT"

echo ""
echo "=== COMPLETE ==="
echo "Report: $REPORT"
echo "Prompts: $TOTAL | Pass: $TOTAL_PASS | Block: $TOTAL_BLOCK | Error: $TOTAL_ERROR"
echo "Match: $TOTAL_MATCH | Mismatch: $TOTAL_MISMATCH"
echo "Cost: \$$COST_DISPLAY / \$$COST_CEILING"

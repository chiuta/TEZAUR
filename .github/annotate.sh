#!/usr/bin/env bash
# Runs a command; on failure publishes the interesting lines and the tail of its output as a workflow annotation
# (annotations stay readable through the API even when raw logs are not). Usage: annotate.sh COMMAND [ARGS...]
log=$(mktemp); "$@" >"$log" 2>&1; rc=$?; cat "$log"
if [ $rc != 0 ]; then
  { echo "exit $rc: $*"; grep -nE "FAIL|rror|panicked|No such|not found|denied|Cannot|cannot" "$log" | head -10; echo "--- tail"; tail -12 "$log"; } | cut -c1-260 | head -c 3600 | awk '{printf "%s%%0A",$0}' | sed 's/^/::error::/'; echo
fi
exit $rc

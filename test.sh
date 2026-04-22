#!/usr/bin/env bash
set -euo pipefail

BASE="http://localhost:8080"
PASS=0
FAIL=0

# error codes (must match ErrorCode enum in src/errors.rs)
CODE_UNAUTHORIZED=1000
CODE_UNKNOWN_QUERY_PARAM=1001
CODE_MISSING_PARAM=1002
CODE_INVALID_PARAM=1003

# -----------------------------------------------------------------------
# helpers
# -----------------------------------------------------------------------

assert_eq() {
    local label="$1" expected="$2" actual="$3"
    if [ "$actual" = "$expected" ]; then
        echo "  PASS: $label"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $label"
        echo "        expected: $expected"
        echo "        actual  : $actual"
        FAIL=$((FAIL + 1))
    fi
}

http_status() {
    curl -s -o /dev/null -w "%{http_code}" "$@"
}

body() {
    curl -s "$@"
}

json_field() {
    local json="$1" field="$2"
    echo "$json" | python3 -c "import sys,json; print(json.load(sys.stdin)['$field'])"
}

# -----------------------------------------------------------------------
# server lifecycle
# -----------------------------------------------------------------------

echo "Starting server..."
./target/debug/remoteapi-test &
SERVER_PID=$!
sleep 1

cleanup() {
    kill "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

# -----------------------------------------------------------------------
# tests
# -----------------------------------------------------------------------

echo ""
echo "--- Authentication ---"

STATUS=$(http_status "$BASE/api/v1/hello")
assert_eq "no auth → 401" "401" "$STATUS"

RESP=$(body "$BASE/api/v1/hello")
assert_eq "no auth body: result=false"  "False" "$(json_field "$RESP" result)"
assert_eq "no auth body: code=$CODE_UNAUTHORIZED" "$CODE_UNAUTHORIZED" "$(json_field "$RESP" code)"

STATUS=$(http_status --digest -u admin:wrong "$BASE/api/v1/hello")
assert_eq "wrong password → 401" "401" "$STATUS"

echo ""
echo "--- v1: unknown params ignored ---"

STATUS=$(http_status --digest -u admin:password "$BASE/api/v1/hello?name=Alice&unknown=foo")
assert_eq "v1 unknown param → 200" "200" "$STATUS"

RESP=$(body --digest -u admin:password "$BASE/api/v1/hello?name=Alice")
assert_eq "v1 result=true"           "True"            "$(json_field "$RESP" result)"
assert_eq "v1 message=Hello, Alice!" "Hello, Alice!"   "$(json_field "$RESP" message)"

RESP=$(body --digest -u admin:password "$BASE/api/v1/hello")
assert_eq "v1 no param → Hello, world!" "Hello, world!" "$(json_field "$RESP" message)"

echo ""
echo "--- v2: unknown params rejected ---"

STATUS=$(http_status --digest -u admin:password "$BASE/api/v2/hello?name=Alice")
assert_eq "v2 known param → 200" "200" "$STATUS"

STATUS=$(http_status --digest -u admin:password "$BASE/api/v2/hello?name=Alice&foo=bar")
assert_eq "v2 unknown param → 400" "400" "$STATUS"

RESP=$(body --digest -u admin:password "$BASE/api/v2/hello?name=Alice&foo=bar")
assert_eq "v2 unknown param: result=false"        "False"               "$(json_field "$RESP" result)"
assert_eq "v2 unknown param: code=$CODE_UNKNOWN_QUERY_PARAM" "$CODE_UNKNOWN_QUERY_PARAM" "$(json_field "$RESP" code)"

RESP=$(body --digest -u admin:password "$BASE/api/v2/hello")
assert_eq "v2 no param → Hello, world!" "Hello, world!" "$(json_field "$RESP" message)"

# -----------------------------------------------------------------------
# summary
# -----------------------------------------------------------------------

echo ""
echo "==============================="
echo "  PASS: $PASS  FAIL: $FAIL"
echo "==============================="
[ "$FAIL" -eq 0 ]

#!/usr/bin/env bash
# Apply the public repository's GitHub settings in one command.
#
#   scripts/public/settings.sh <owner/repo>                 print every gh api call (dry run)
#   scripts/public/settings.sh <owner/repo> --apply         run them
#   scripts/public/settings.sh <owner/repo> --apply --prune also delete labels not in .github/labels.yml
#
# The dry run makes no network call and needs neither gh nor a token. --apply needs
# gh authenticated as a repository administrator. Running --apply twice changes
# nothing the second time: settings are PATCH/PUT to fixed values, the ruleset is
# updated in place when one with the same name exists, and labels are written only
# when missing or different.
set -euo pipefail

# --- Configuration -----------------------------------------------------------------
DESCRIPTION='Static analyzer for JavaScript and TypeScript test suites: finds tests that can pass without testing anything. Runs locally, never runs your tests.'
HOMEPAGE='https://spirolo.dev'
TOPICS=(testing javascript typescript linter rust static-analysis test-quality)
RULESET_NAME='main'
# Check names reported by the jobs of the public .github/workflows/ci.yml
# (scripts/public/files/.github/workflows/ci.yml here; job `name:`, matrix expanded).
REQUIRED_CHECKS=(
  'build and test'
)
# -----------------------------------------------------------------------------------

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
LABELS_FILE="$ROOT/.github/labels.yml"
CODEQL_WORKFLOW="$ROOT/.github/workflows/codeql.yml"

usage() {
  sed -n '2,7p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

APPLY=0 PRUNE=0 REPO=''
for arg in "$@"; do
  case "$arg" in
    --apply) APPLY=1 ;;
    --prune) PRUNE=1 ;;
    -h|--help) usage 0 ;;
    -*) echo "unknown option: $arg" >&2; usage 2 >&2 ;;
    *)
      [ -z "$REPO" ] || { echo "only one <owner/repo> is accepted" >&2; exit 2; }
      REPO=$arg ;;
  esac
done
[ -n "$REPO" ] || usage 2 >&2
[[ "$REPO" =~ ^[A-Za-z0-9-]+/[A-Za-z0-9._-]+$ ]] || { echo "not an owner/repo: $REPO" >&2; exit 2; }

# Shell-quote one argument for printing, leaving plain words readable.
quote() {
  if [[ "$1" =~ ^[A-Za-z0-9_./:=@%+,-]+$ ]]; then printf '%s' "$1"
  else printf "'%s'" "${1//\'/\'\\\'\'}"
  fi
}

# Print a gh api call; with --apply also run it. A JSON body, when given, goes to stdin.
#   call [--body JSON] <gh api arguments...>
call() {
  local body=''
  if [ "$1" = --body ]; then body=$2; shift 2; fi
  local line='gh api' arg
  for arg in "$@"; do line+=" $(quote "$arg")"; done
  if [ -n "$body" ]; then
    printf '%s --input - <<'"'"'JSON'"'"'\n%s\nJSON\n' "$line" "$body"
  else
    printf '%s\n' "$line"
  fi
  [ "$APPLY" = 1 ] || return 0
  if [ -n "$body" ]; then
    printf '%s' "$body" | gh api "$@" --input - >/dev/null
  else
    gh api "$@" >/dev/null
  fi
}

note() { printf '# %s\n' "$*"; }

json_string() {
  local s=${1//\\/\\\\}
  s=${s//\"/\\\"}
  printf '"%s"' "$s"
}

urlencode() {
  local LC_ALL=C s=$1 out='' c i
  for ((i = 0; i < ${#s}; i++)); do
    c=${s:i:1}
    case "$c" in
      [A-Za-z0-9._~-]) out+=$c ;;
      *) out+=$(printf '%%%02X' "'$c") ;;
    esac
  done
  printf '%s' "$out"
}

# Labels from .github/labels.yml as name<TAB>color<TAB>description lines.
read_labels() {
  awk '
    function value(line) { sub(/^[^"]*"/, "", line); sub(/"[[:space:]]*$/, "", line); return line }
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    /^- name: "[^"]+"[[:space:]]*$/ { name = value($0); next }
    /^  color: "[0-9A-Fa-f]{6}"[[:space:]]*$/ { color = value($0); next }
    /^  description: "[^"]*"[[:space:]]*$/ {
      if (name == "" || color == "") { bad = 1; exit }
      printf "%s\t%s\t%s\n", name, color, value($0); name = ""; color = ""; next
    }
    { bad = 1; exit }
    END { if (bad || name != "") exit 1 }
  ' "$LABELS_FILE" || { echo "cannot parse $LABELS_FILE" >&2; exit 1; }
}

if [ "$APPLY" = 1 ]; then
  command -v gh >/dev/null || { echo "gh is required for --apply" >&2; exit 1; }
  gh api "repos/$REPO" --jq .full_name >/dev/null
else
  note "dry run for $REPO: nothing is sent. Re-run with --apply to send these calls."
fi

# Repository settings: features, merge commits only, tidy branches.
call -X PATCH "repos/$REPO" \
  -f "description=$DESCRIPTION" -f "homepage=$HOMEPAGE" \
  -F has_issues=true -F has_discussions=true -F has_wiki=false -F has_projects=false \
  -F allow_merge_commit=true -F allow_squash_merge=false -F allow_rebase_merge=false \
  -F delete_branch_on_merge=true -F allow_auto_merge=false -F allow_update_branch=true

topic_args=()
for topic in "${TOPICS[@]}"; do topic_args+=(-f "names[]=$topic"); done
call -X PUT "repos/$REPO/topics" "${topic_args[@]}"

# Security: Dependabot alerts and fixes, secret scanning with push protection, private reports.
call -X PUT "repos/$REPO/vulnerability-alerts"
call -X PUT "repos/$REPO/automated-security-fixes"
call --body '{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"enabled"}}}' \
  -X PATCH "repos/$REPO"
call -X PUT "repos/$REPO/private-vulnerability-reporting"

# Actions: read-only default token, and workflows may not approve pull requests.
call -X PUT "repos/$REPO/actions/permissions/workflow" \
  -f default_workflow_permissions=read -F can_approve_pull_request_reviews=false
call -X PUT "repos/$REPO/actions/permissions/fork-pr-contributor-approval" \
  -f approval_policy=first_time_contributors

# Default branch ruleset: pull request, the ci checks, no force-push, no deletion.
checks=''
for check in "${REQUIRED_CHECKS[@]}"; do
  checks+="${checks:+,}{\"context\":$(json_string "$check")}"
done
ruleset="{\"name\":$(json_string "$RULESET_NAME"),\"target\":\"branch\",\"enforcement\":\"active\",\
\"conditions\":{\"ref_name\":{\"include\":[\"~DEFAULT_BRANCH\"],\"exclude\":[]}},\
\"bypass_actors\":[{\"actor_id\":5,\"actor_type\":\"RepositoryRole\",\"bypass_mode\":\"always\"}],\
\"rules\":[{\"type\":\"deletion\"},{\"type\":\"non_fast_forward\"},\
{\"type\":\"pull_request\",\"parameters\":{\"required_approving_review_count\":0,\"dismiss_stale_reviews_on_push\":true,\
\"require_code_owner_review\":false,\"require_last_push_approval\":false,\"required_review_thread_resolution\":true}},\
{\"type\":\"required_status_checks\",\"parameters\":{\"strict_required_status_checks_policy\":false,\
\"required_status_checks\":[$checks]}}]}"
ruleset_id=''
if [ "$APPLY" = 1 ]; then
  ruleset_id=$(gh api "repos/$REPO/rulesets?includes_parents=false" \
    --jq ".[] | select(.name == $(json_string "$RULESET_NAME")) | .id" | head -n 1)
else
  note "a ruleset named $RULESET_NAME that already exists is updated in place: PUT repos/$REPO/rulesets/<id>"
fi
if [ -n "$ruleset_id" ]; then
  call --body "$ruleset" -X PUT "repos/$REPO/rulesets/$ruleset_id"
else
  call --body "$ruleset" -X POST "repos/$REPO/rulesets"
fi

# Code scanning: the committed CodeQL workflow wins over default setup.
if [ -f "$CODEQL_WORKFLOW" ]; then
  note "code scanning default setup skipped: .github/workflows/codeql.yml runs CodeQL"
else
  call -X PATCH "repos/$REPO/code-scanning/default-setup" -f state=configured -f query_suite=default
fi

# Labels: create missing ones, update changed ones, delete unlisted ones only with --prune.
existing=''
if [ "$APPLY" = 1 ]; then
  existing=$(gh api --paginate "repos/$REPO/labels?per_page=100" \
    --jq '.[] | [.name, (.color | ascii_downcase), (.description // "")] | @tsv')
else
  note "labels from .github/labels.yml: POST when missing, PATCH repos/$REPO/labels/<name> when changed, unchanged ones are skipped"
fi
wanted=$(read_labels)
while IFS=$'\t' read -r name color description; do
  [ -n "$name" ] || continue
  current=$(printf '%s\n' "$existing" | awk -F '\t' -v n="$name" '$1 == n { print; exit }')
  lower=$(printf '%s' "$color" | tr 'A-F' 'a-f')
  if [ -z "$current" ]; then
    call -X POST "repos/$REPO/labels" -f "name=$name" -f "color=$color" -f "description=$description"
  elif [ "$current" != "$name"$'\t'"$lower"$'\t'"$description" ]; then
    call -X PATCH "repos/$REPO/labels/$(urlencode "$name")" -f "color=$color" -f "description=$description"
  fi
done <<<"$wanted"
if [ "$PRUNE" = 1 ]; then
  if [ "$APPLY" = 1 ]; then
    while IFS=$'\t' read -r name _; do
      [ -n "$name" ] || continue
      printf '%s\n' "$wanted" | awk -F '\t' -v n="$name" '$1 == n { found = 1 } END { exit !found }' \
        || call -X DELETE "repos/$REPO/labels/$(urlencode "$name")"
    done <<<"$existing"
  else
    note "--prune: DELETE repos/$REPO/labels/<name> for every label not in .github/labels.yml"
  fi
fi

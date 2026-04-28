#!/usr/bin/env bash
# Fetch Sieve RFC and draft texts for local development.
# Files are placed under docs/ which is git-ignored.
# See docs/SOURCES.md for the canonical source list.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."

fetch() {
    local url="$1"
    local dest="$2"
    mkdir -p "$(dirname "$dest")"
    if [ -f "$dest" ]; then
        echo "  already exists: $dest"
    else
        echo "  fetching: $url"
        curl -sSfL "$url" -o "$dest"
    fi
}

echo "Fetching base standards..."
fetch https://www.rfc-editor.org/rfc/rfc5228.txt "$ROOT/docs/rfc/rfc5228-base.txt"
fetch https://www.rfc-editor.org/rfc/rfc5229.txt "$ROOT/docs/rfc/rfc5229-variables.txt"
fetch https://www.ietf.org/archive/id/draft-murchison-sieve-regex-07.txt "$ROOT/docs/drafts/draft-murchison-sieve-regex-07.txt"

echo "Fetching IETF extension RFCs..."
fetch https://www.rfc-editor.org/rfc/rfc3894.txt "$ROOT/docs/rfc/rfc3894-copy.txt"
fetch https://www.rfc-editor.org/rfc/rfc5173.txt "$ROOT/docs/rfc/rfc5173-body.txt"
fetch https://www.rfc-editor.org/rfc/rfc5183.txt "$ROOT/docs/rfc/rfc5183-environment.txt"
fetch https://www.rfc-editor.org/rfc/rfc5230.txt "$ROOT/docs/rfc/rfc5230-vacation.txt"
fetch https://www.rfc-editor.org/rfc/rfc5231.txt "$ROOT/docs/rfc/rfc5231-relational.txt"
fetch https://www.rfc-editor.org/rfc/rfc5232.txt "$ROOT/docs/rfc/rfc5232-imap4flags.txt"
fetch https://www.rfc-editor.org/rfc/rfc5233.txt "$ROOT/docs/rfc/rfc5233-subaddress.txt"
fetch https://www.rfc-editor.org/rfc/rfc5235.txt "$ROOT/docs/rfc/rfc5235-spamtest-virustest.txt"
fetch https://www.rfc-editor.org/rfc/rfc5260.txt "$ROOT/docs/rfc/rfc5260-date-index.txt"
fetch https://www.rfc-editor.org/rfc/rfc5293.txt "$ROOT/docs/rfc/rfc5293-editheader.txt"
fetch https://www.rfc-editor.org/rfc/rfc5429.txt "$ROOT/docs/rfc/rfc5429-reject-ereject.txt"
fetch https://www.rfc-editor.org/rfc/rfc5435.txt "$ROOT/docs/rfc/rfc5435-enotify.txt"
fetch https://www.rfc-editor.org/rfc/rfc5436.txt "$ROOT/docs/rfc/rfc5436-notify-mailto.txt"
fetch https://www.rfc-editor.org/rfc/rfc5463.txt "$ROOT/docs/rfc/rfc5463-ihave.txt"
fetch https://www.rfc-editor.org/rfc/rfc5490.txt "$ROOT/docs/rfc/rfc5490-mailbox-metadata.txt"
fetch https://www.rfc-editor.org/rfc/rfc5703.txt "$ROOT/docs/rfc/rfc5703-mime-part-tests.txt"
fetch https://www.rfc-editor.org/rfc/rfc6009.txt "$ROOT/docs/rfc/rfc6009-dsn-deliverby.txt"
fetch https://www.rfc-editor.org/rfc/rfc6131.txt "$ROOT/docs/rfc/rfc6131-vacation-seconds.txt"
fetch https://www.rfc-editor.org/rfc/rfc6134.txt "$ROOT/docs/rfc/rfc6134-extlists.txt"
fetch https://www.rfc-editor.org/rfc/rfc6609.txt "$ROOT/docs/rfc/rfc6609-include.txt"
fetch https://www.rfc-editor.org/rfc/rfc6785.txt "$ROOT/docs/rfc/rfc6785-imapsieve.txt"
fetch https://www.rfc-editor.org/rfc/rfc7352.txt "$ROOT/docs/rfc/rfc7352-duplicate.txt"
fetch https://www.rfc-editor.org/rfc/rfc8579.txt "$ROOT/docs/rfc/rfc8579-special-use.txt"
fetch https://www.rfc-editor.org/rfc/rfc8580.txt "$ROOT/docs/rfc/rfc8580-fcc.txt"
fetch https://www.rfc-editor.org/rfc/rfc9671.txt "$ROOT/docs/rfc/rfc9671-processcalendar.txt"

echo "Fetching active drafts..."
fetch https://www.ietf.org/archive/id/draft-ietf-extra-sieve-snooze-07.txt "$ROOT/docs/drafts/draft-ietf-extra-sieve-snooze-07.txt"
fetch https://www.ietf.org/archive/id/draft-ietf-extra-sieve-mailboxid-09.txt "$ROOT/docs/drafts/draft-ietf-extra-sieve-mailboxid-09.txt"

echo "Done. Files are in docs/rfc/ and docs/drafts/ (git-ignored)."

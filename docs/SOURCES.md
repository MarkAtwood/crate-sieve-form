# Sieve Specification Sources

Reference documents for Sieve extensions tracked in this project.
These files are **not versioned** — fetch locally for development using the
URLs below.  A convenience script is provided at `scripts/fetch-docs.sh`.

## Base Standards

| RFC / Draft | Extension | URL |
|-------------|-----------|-----|
| RFC 5228 | base language | https://www.rfc-editor.org/rfc/rfc5228.txt |
| RFC 5229 | `variables` | https://www.rfc-editor.org/rfc/rfc5229.txt |
| draft-murchison-sieve-regex-07 | `regex` | https://www.ietf.org/archive/id/draft-murchison-sieve-regex-07.txt |

## IETF RFC Extensions

| RFC | Extension | URL |
|-----|-----------|-----|
| RFC 3894 | `copy` | https://www.rfc-editor.org/rfc/rfc3894.txt |
| RFC 5173 | `body` | https://www.rfc-editor.org/rfc/rfc5173.txt |
| RFC 5183 | `environment` | https://www.rfc-editor.org/rfc/rfc5183.txt |
| RFC 5230 | `vacation` | https://www.rfc-editor.org/rfc/rfc5230.txt |
| RFC 5231 | `relational` | https://www.rfc-editor.org/rfc/rfc5231.txt |
| RFC 5232 | `imap4flags` | https://www.rfc-editor.org/rfc/rfc5232.txt |
| RFC 5233 | `subaddress` | https://www.rfc-editor.org/rfc/rfc5233.txt |
| RFC 5235 | `spamtest`, `virustest` | https://www.rfc-editor.org/rfc/rfc5235.txt |
| RFC 5260 | `date`, `index` | https://www.rfc-editor.org/rfc/rfc5260.txt |
| RFC 5293 | `editheader` | https://www.rfc-editor.org/rfc/rfc5293.txt |
| RFC 5429 | `reject`, `ereject` | https://www.rfc-editor.org/rfc/rfc5429.txt |
| RFC 5435 | `enotify` | https://www.rfc-editor.org/rfc/rfc5435.txt |
| RFC 5436 | (notify method: mailto) | https://www.rfc-editor.org/rfc/rfc5436.txt |
| RFC 5463 | `ihave` | https://www.rfc-editor.org/rfc/rfc5463.txt |
| RFC 5490 | `mailbox`, `mboxmetadata`, `servermetadata` | https://www.rfc-editor.org/rfc/rfc5490.txt |
| RFC 5703 | `mime`, `foreverypart`, `replace`, `enclose`, `extracttext` | https://www.rfc-editor.org/rfc/rfc5703.txt |
| RFC 6009 | `envelope-dsn`, `redirect-dsn`, `envelope-deliverby`, `redirect-deliverby` | https://www.rfc-editor.org/rfc/rfc6009.txt |
| RFC 6131 | `vacation-seconds` | https://www.rfc-editor.org/rfc/rfc6131.txt |
| RFC 6134 | `extlists` | https://www.rfc-editor.org/rfc/rfc6134.txt |
| RFC 6609 | `include` | https://www.rfc-editor.org/rfc/rfc6609.txt |
| RFC 6785 | `imapsieve` | https://www.rfc-editor.org/rfc/rfc6785.txt |
| RFC 7352 | `duplicate` | https://www.rfc-editor.org/rfc/rfc7352.txt |
| RFC 8579 | `special-use` | https://www.rfc-editor.org/rfc/rfc8579.txt |
| RFC 8580 | `fcc` | https://www.rfc-editor.org/rfc/rfc8580.txt |
| RFC 9671 | `processcalendar` | https://www.rfc-editor.org/rfc/rfc9671.txt |

## Active IETF Drafts

| Draft | Extension | URL |
|-------|-----------|-----|
| draft-ietf-extra-sieve-snooze | `snooze` | https://www.ietf.org/archive/id/draft-ietf-extra-sieve-snooze-07.txt |
| draft-ietf-extra-sieve-mailboxid | `mailboxid` | https://www.ietf.org/archive/id/draft-ietf-extra-sieve-mailboxid-09.txt |

## Vendor Extensions

| Vendor | Extension | Reference |
|--------|-----------|-----------|
| Dovecot | `vnd.dovecot.debug` | https://doc.dovecot.org/configuration_manual/sieve/plugins/debug/ |
| Dovecot | `vnd.dovecot.pipe`, `vnd.dovecot.filter`, `vnd.dovecot.execute` | https://doc.dovecot.org/configuration_manual/sieve/plugins/extprograms/ |
| Dovecot | `vnd.dovecot.environment` | https://doc.dovecot.org/configuration_manual/sieve/plugins/environment/ |
| Dovecot | `vnd.dovecot.report` | https://doc.dovecot.org/configuration_manual/sieve/plugins/report/ |
| Cyrus / Fastmail | `vnd.cyrus.log`, `vnd.cyrus.jmapquery`, `vnd.cyrus.snooze`, `vnd.cyrus.implicit_keep_target`, `vnd.cyrus.imip` | https://www.cyrusimap.org/imap/reference/manpages/systemcommands/sieve.html |
| ProtonMail | `vnd.proton.expire`, `vnd.proton.eval` | https://proton.me/support/sieve-advanced-custom-filters |
| Fastmail | (uses `vnd.cyrus.*`) | https://www.fastmail.com/help/technical/sieve-guide.html |

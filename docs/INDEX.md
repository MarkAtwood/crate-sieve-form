# Sieve Specification Documents

Reference documents for all Sieve extensions tracked in this project.

## Base Standards

| File | RFC | Description |
|------|-----|-------------|
| [rfc/rfc5228-base.txt](rfc/rfc5228-base.txt) | RFC 5228 | Sieve: An Email Filtering Language (base) |
| [rfc/rfc5229-variables.txt](rfc/rfc5229-variables.txt) | RFC 5229 | Variables Extension |
| [drafts/draft-murchison-sieve-regex-07.txt](drafts/draft-murchison-sieve-regex-07.txt) | draft | Regular Expression Extension |

## IETF RFC Extensions

| File | RFC | Extension name | Description |
|------|-----|----------------|-------------|
| [rfc/rfc3894-copy.txt](rfc/rfc3894-copy.txt) | RFC 3894 | `copy` | `:copy` argument for fileinto/redirect |
| [rfc/rfc5173-body.txt](rfc/rfc5173-body.txt) | RFC 5173 | `body` | Body content tests |
| [rfc/rfc5183-environment.txt](rfc/rfc5183-environment.txt) | RFC 5183 | `environment` | Interpreter/system environment access |
| [rfc/rfc5230-vacation.txt](rfc/rfc5230-vacation.txt) | RFC 5230 | `vacation` | Auto-reply vacation messages |
| [rfc/rfc5231-relational.txt](rfc/rfc5231-relational.txt) | RFC 5231 | `relational` | Relational and numeric comparisons |
| [rfc/rfc5232-imap4flags.txt](rfc/rfc5232-imap4flags.txt) | RFC 5232 | `imap4flags` | IMAP flag manipulation |
| [rfc/rfc5233-subaddress.txt](rfc/rfc5233-subaddress.txt) | RFC 5233 | `subaddress` | Subaddress detail testing |
| [rfc/rfc5235-spamtest-virustest.txt](rfc/rfc5235-spamtest-virustest.txt) | RFC 5235 | `spamtest`, `virustest` | Spam/virus score tests |
| [rfc/rfc5260-date-index.txt](rfc/rfc5260-date-index.txt) | RFC 5260 | `date`, `index` | Date/time tests and header indexing |
| [rfc/rfc5293-editheader.txt](rfc/rfc5293-editheader.txt) | RFC 5293 | `editheader` | Add and delete message headers |
| [rfc/rfc5429-reject-ereject.txt](rfc/rfc5429-reject-ereject.txt) | RFC 5429 | `reject`, `ereject` | SMTP-level message rejection |
| [rfc/rfc5435-enotify.txt](rfc/rfc5435-enotify.txt) | RFC 5435 | `enotify` | External notifications |
| [rfc/rfc5436-notify-mailto.txt](rfc/rfc5436-notify-mailto.txt) | RFC 5436 | (notify method) | Notify method: mailto |
| [rfc/rfc5463-ihave.txt](rfc/rfc5463-ihave.txt) | RFC 5463 | `ihave` | Runtime capability checking |
| [rfc/rfc5490-mailbox-metadata.txt](rfc/rfc5490-mailbox-metadata.txt) | RFC 5490 | `mailbox`, `mboxmetadata`, `servermetadata` | Mailbox existence and metadata |
| [rfc/rfc5703-mime-part-tests.txt](rfc/rfc5703-mime-part-tests.txt) | RFC 5703 | `mime`, `foreverypart`, `replace`, `enclose`, `extracttext` | MIME part tests and manipulation |
| [rfc/rfc6009-dsn-deliverby.txt](rfc/rfc6009-dsn-deliverby.txt) | RFC 6009 | `envelope-dsn`, `redirect-dsn`, `envelope-deliverby`, `redirect-deliverby` | DSN and Deliver-By envelope access |
| [rfc/rfc6131-vacation-seconds.txt](rfc/rfc6131-vacation-seconds.txt) | RFC 6131 | `vacation-seconds` | Vacation with sub-day intervals |
| [rfc/rfc6134-extlists.txt](rfc/rfc6134-extlists.txt) | RFC 6134 | `extlists` | Externally-stored list matching |
| [rfc/rfc6609-include.txt](rfc/rfc6609-include.txt) | RFC 6609 | `include` | Script inclusion |
| [rfc/rfc6785-imapsieve.txt](rfc/rfc6785-imapsieve.txt) | RFC 6785 | `imapsieve` | IMAP event triggers |
| [rfc/rfc7352-duplicate.txt](rfc/rfc7352-duplicate.txt) | RFC 7352 | `duplicate` | Duplicate delivery detection |
| [rfc/rfc8579-special-use.txt](rfc/rfc8579-special-use.txt) | RFC 8579 | `special-use` | Delivery to special-use mailboxes |
| [rfc/rfc8580-fcc.txt](rfc/rfc8580-fcc.txt) | RFC 8580 | `fcc` | File carbon copy for generated messages |
| [rfc/rfc9671-processcalendar.txt](rfc/rfc9671-processcalendar.txt) | RFC 9671 | `processcalendar` | iMIP calendar attachment processing |

## Active IETF Drafts

| File | Draft | Extension name | Description |
|------|-------|----------------|-------------|
| [drafts/draft-ietf-extra-sieve-snooze-07.txt](drafts/draft-ietf-extra-sieve-snooze-07.txt) | draft-ietf-extra-sieve-snooze | `snooze` | Postpone message delivery to a later time |
| [drafts/draft-ietf-extra-sieve-mailboxid-09.txt](drafts/draft-ietf-extra-sieve-mailboxid-09.txt) | draft-ietf-extra-sieve-mailboxid | `mailboxid` | Deliver by stable IMAP mailbox ID |

## Dovecot/Pigeonhole Vendor Extensions

The extprograms spec covers `vnd.dovecot.pipe`, `vnd.dovecot.filter`, and `vnd.dovecot.execute`.

| File | Extension name | Description |
|------|----------------|-------------|
| [vendor/dovecot/spec-vnd.dovecot.debug.txt](vendor/dovecot/spec-vnd.dovecot.debug.txt) | `vnd.dovecot.debug` | `debug_log` action: write to Dovecot log |
| [vendor/dovecot/spec-vnd.dovecot.extprograms.txt](vendor/dovecot/spec-vnd.dovecot.extprograms.txt) | `vnd.dovecot.pipe`, `vnd.dovecot.filter`, `vnd.dovecot.execute` | External program integration |
| [vendor/dovecot/spec-vnd.dovecot.environment.txt](vendor/dovecot/spec-vnd.dovecot.environment.txt) | `vnd.dovecot.environment` | Dovecot-specific environment items |
| [vendor/dovecot/spec-vnd.dovecot.report.txt](vendor/dovecot/spec-vnd.dovecot.report.txt) | `vnd.dovecot.report` | ARF/MARF abuse reporting |

## Cyrus IMAP Vendor Extensions

| File | Extension name | Description |
|------|----------------|-------------|
| [vendor/cyrus/sieve.md](vendor/cyrus/sieve.md) | `vnd.cyrus.log`, `vnd.cyrus.jmapquery`, `vnd.cyrus.snooze`, `vnd.cyrus.implicit_keep_target`, `vnd.cyrus.imip` | Cyrus-specific Sieve extensions |
| [vendor/cyrus/sieve-reference.html](vendor/cyrus/sieve-reference.html) | (all) | cyrusimap.org reference page (HTML) |

Note: `vnd.cyrus.imip` is a legacy alias for `processcalendar` (RFC 9671).
Fastmail runs Cyrus IMAP and supports all `vnd.cyrus.*` extensions above.

## ProtonMail Vendor Extensions

| File | Extension name | Description |
|------|----------------|-------------|
| [vendor/protonmail/sieve-extensions.md](vendor/protonmail/sieve-extensions.md) | `vnd.proton.expire`, `vnd.proton.eval` | ProtonMail-specific Sieve extensions |

## Fastmail

| File | Description |
|------|-------------|
| [vendor/fastmail/sieve-support.md](vendor/fastmail/sieve-support.md) | Fastmail supported extensions, notes, and Fastmail-specific headers |

Note: Fastmail uses `vnd.cyrus.*` extensions — there is no `vnd.fastmail.*` namespace.

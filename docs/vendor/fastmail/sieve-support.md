# Fastmail Sieve Support

Sources:
- https://www.fastmail.help/hc/en-us/articles/1500000280481-Using-Sieve-scripts-in-Fastmail
- https://www.fastmail.help/hc/en-us/articles/360058753814-Sieve-frequently-asked-questions
- https://www.fastmail.help/hc/en-us/articles/360058753794-Sieve-examples
Retrieved: 2026-04-24

Fastmail runs Cyrus IMAP. All vendor-specific Sieve extensions use the
`vnd.cyrus.*` namespace — there is no `vnd.fastmail.*` namespace.

Sieve tester: https://app.fastmail.com/sievetester

---

## Supported extensions

Standard:
`fileinto`, `reject`, `vacation`, `vacation-seconds`, `envelope`, `body`,
`relational`, `regex`, `subaddress`, `copy`, `mailbox`, `mboxmetadata`,
`servermetadata`, `date`, `index`, `comparator-i;ascii-numeric`,
`variables`, `imap4flags`, `editheader`, `duplicate`, `special-use`,
`mailboxid`, `fcc`

Cyrus vendor:
`vnd.cyrus.jmapquery`, `vnd.cyrus.log`, `vnd.cyrus.snooze`, `vnd.cyrus.imip`

See [../cyrus/](../cyrus/) for Cyrus extension specs.

---

## Spam filtering via Sieve

Fastmail does **not** expose `spamtest` (RFC 5235), `virustest` (RFC 5235),
or `environment` (RFC 5183) to users, even though Cyrus IMAP supports them.
RFC 5235 is not fully implemented in Cyrus as of this writing.

Instead, Fastmail adds X-Spam-* headers (SpamAssassin-based) which you can
test with the standard `header` test:

| Header | Format | Notes |
|--------|--------|-------|
| `X-Spam-score` | Decimal, e.g. `5.5` | Aggregate score; default spam threshold is 5.0 |
| `X-Spam-hits` | SpamAssassin rule list, e.g. `BAYES_99 3.5, HTML_MESSAGE 0.001` | Rules that fired |
| `X-Spam-source` | String | Calculated source from Received headers |
| `X-Spam-charsets` | String | Character sets detected |
| `X-Spam-known-sender` | `yes` / absent | Sender in address book; prevents spam folder |
| `X-Spam-sender-reputation` | Integer 0–1000 | Past interaction score |

Not all headers appear on every message.

Example — file high-scoring spam into a custom folder rather than Spam:

```sieve
require ["fileinto", "relational", "comparator-i;ascii-numeric"];
if header :value "ge" :comparator "i;ascii-numeric" "X-Spam-score" "8" {
    fileinto "HighSpam";
}
```

Note: `X-Spam-score` is an unbounded SpamAssassin score, **not** the 0–10
normalized scale that RFC 5235 spamtest defines. They are not interchangeable.

ProtonMail, by contrast, exposes a spam threshold via the `environment`
extension item `vnd.proton.spam-threshold` and uses `spamtest` against it
(see `../protonmail/sieve-extensions.md`).

---

## Notes

- Filters run on incoming mail only; outgoing mail is not filtered.
- Scripts run server-side regardless of client (web, IMAP, POP).
- ManageSieve protocol is not supported; scripts must be edited via the web UI.
- Folder names are case-sensitive.
- Multiple `fileinto` calls stack (message filed into multiple folders).
- `fileinto :copy` (RFC 3894) is supported.
- Push notifications: `addflag "$notify";`
- `imap4flags` flags: `\\Seen`, `\\Deleted`, `\\Flagged`
- Sieve cannot reject before SMTP delivery completes (no pre-DATA rejection).

---

## Fastmail-specific headers

Fastmail adds delivery metadata headers useful for filtering:

- `X-Delivered-To` — the address the message was delivered to (useful with
  catch-all accounts to route by recipient alias)

Example using X-Delivered-To:

```sieve
require ["fileinto", "imap4flags"];
if address :is "X-Delivered-To" "work@example.com" {
    fileinto "Work";
} elsif address :is "X-Delivered-To" "lists@example.com" {
    setflag "\\Seen";
    fileinto "Lists";
}
```

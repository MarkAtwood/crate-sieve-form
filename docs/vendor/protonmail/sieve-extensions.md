# ProtonMail Sieve Extensions

Source: https://proton.me/support/sieve-advanced-custom-filters
Retrieved: 2026-04-24

ProtonMail runs its own Sieve implementation with two proprietary extensions:
`vnd.proton.expire` and `vnd.proton.eval`. Both are documented on the page above.

---

## vnd.proton.expire

Capability string: `vnd.proton.expire`

Sets a TTL on a message. ProtonMail auto-deletes the message after the
specified duration. Maximum is 730 days (auto-capped).

### Actions

**expire** — set expiration

```sieve
require "vnd.proton.expire";
expire "day" "10";
```

Units: `"day"`, `"minute"`, `"second"`

**unexpire** — remove expiration

```sieve
require "vnd.proton.expire";
unexpire;
```

### Tests

**hasexpiration** — true if message has an expiration set

```sieve
if hasexpiration { ... }
```

**expiration** — compare expiration time numerically

```sieve
require ["vnd.proton.expire", "relational", "comparator-i;ascii-numeric"];
if expiration :comparator "i;ascii-numeric" :value "le" "day" "5" { ... }
```

### Examples

```sieve
require ["extlists", "vnd.proton.expire"];
if not anyof(
    header :list "from" ":addrbook:personal",
    header :list "from" ":addrbook:myself"
) {
    expire "day" "10";
}
```

---

## vnd.proton.eval

Capability string: `vnd.proton.eval`

Adds a `:eval` modifier to the `set` action from the `variables` extension
(RFC 5229). Evaluates an arithmetic expression and stores the result as a
string variable. Also implies the `:length` modifier (though `:length` is
standard in RFC 5229).

Operators: `+`, `-`, `*`, `/`

Variables are substituted before evaluation using the standard `${name}` syntax.

### Syntax

```sieve
require ["variables", "vnd.proton.eval"];
set :eval "varname" "expression";
```

### Example

```sieve
require ["variables", "vnd.proton.eval"];
if header :matches "from" "*" {
    set :length "length" "${1}";
    set :eval "result" "${length} * 25 - 1 / 8 + 3";
    fileinto "${result}";
}
```

---

---

## vnd.proton.spam-threshold (environment item, not an extension)

This is **not** a separate Sieve extension. ProtonMail exposes their internal
spam score as an environment item named `vnd.proton.spam-threshold`, accessible
via the standard `environment` extension (RFC 5183). Their generated filter
scripts (V2) use `spamtest` (RFC 5235) against this value:

```sieve
require ["include", "environment", "variables", "relational",
         "comparator-i;ascii-numeric", "spamtest"];

# Generated: Do not run this script on spam messages
if allof (environment :matches "vnd.proton.spam-threshold" "*",
          spamtest :value "ge" :comparator "i;ascii-numeric" "${1}") {
    return;
}
```

The pattern captures the threshold via `${1}` from the `:matches` test, then
compares the spamtest score against it. This guard runs before any user rules.

Source: `webclient-sieve-package/src/constants.ts` and fixture files in
`webclient-sieve-package/fixtures/`.

---

## Supported standard extensions

ProtonMail also supports these standard extensions (not exhaustive):

`date`, `envelope`, `fileinto`, `imap4flags`, `reject`, `vacation`,
`variables`, `relational`, `regex`, `comparator-i;ascii-numeric`,
`extlists`, `include`

### ProtonMail-specific extlists URIs

The `extlists` extension (RFC 6134) is wired to ProtonMail's contact store:

| URI | Contents |
|-----|----------|
| `:addrbook:personal` | Personal contacts |
| `:addrbook:personal?label=GroupName` | Specific contact group |
| `:addrbook:myself` | Your own addresses |
| `:addrbook:organization` | Organization members |
| `:incomingdefaults:inbox` | Allow List |
| `:incomingdefaults:spam` | Block List |

Query parameters on `:addrbook:personal`:
- `?label.starts-with=`, `?label.ends-with=`, `?label.contains=`
- `?keypinning=true/false`
- `?encryption=true/false`
- `?signing=true/false`

### Envelope note

In ProtonMail: envelope `from` = Return-Path header; envelope `to` =
X-Original-To header.

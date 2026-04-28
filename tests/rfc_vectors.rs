// SPDX-License-Identifier: MIT
//
// Integration tests with static test vectors from:
//   - RFC 5228 (Sieve: An Email Filtering Language), Appendix A
//   - RFC 5229 (Sieve Email Filtering: Variables Extension), §5
//   - Hand-traced expected outputs (independent of the implementation)
//
// These tests use only the public API: compile() and evaluate().
// Expected outputs are hardcoded — never derived from the code under test.

use sieve_form::{compile, evaluate, SieveAction};

// ---------------------------------------------------------------------------
// Minimal RFC 2822 message builders
// ---------------------------------------------------------------------------

/// Build a minimal RFC 2822 message with specified headers and body.
fn make_message(from: &str, to: &str, subject: &str, body: &str) -> Vec<u8> {
    format!("From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\n\r\n{body}\r\n").into_bytes()
}

/// Build a message with an extra arbitrary header.
fn make_message_with_header(extra_name: &str, extra_value: &str) -> Vec<u8> {
    format!(
        "From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: Test\r\n{extra_name}: {extra_value}\r\n\r\nBody.\r\n"
    )
    .into_bytes()
}

// ---------------------------------------------------------------------------
// RFC 5228 §8 — basic action tests
// These are the simplest possible scripts: a single action, no condition.
// Expected output is trivially hand-traceable from the RFC action semantics.
// ---------------------------------------------------------------------------

/// RFC 5228 §4.1 — keep: explicit keep action returns [Keep].
#[test]
fn test_keep_action() {
    let script = compile(b"keep;").expect("compile failed");
    let msg = make_message(
        "alice@example.com",
        "bob@example.com",
        "Hello",
        "Body text.",
    );
    let actions = evaluate(&script, &msg, "alice@example.com", "bob@example.com");
    // RFC 5228 §4.1: keep deposits the message in the default mailbox.
    assert_eq!(actions, vec![SieveAction::Keep]);
}

/// RFC 5228 §4.2 — fileinto: routes message to a named folder.
/// The require ["fileinto"] declaration is mandatory per RFC 5228 §2.6.
#[test]
fn test_fileinto() {
    let script =
        compile(b"require [\"fileinto\"]; fileinto \"INBOX.spam\";").expect("compile failed");
    let msg = make_message(
        "spammer@example.net",
        "victim@example.com",
        "You won",
        "Click here.",
    );
    let actions = evaluate(&script, &msg, "spammer@example.net", "victim@example.com");
    // fileinto "INBOX.spam" must route to exactly that mailbox name.
    assert_eq!(
        actions,
        vec![SieveAction::FileInto("INBOX.spam".to_string())]
    );
}

/// RFC 5228 §4.3 — redirect is not implemented; this tests discard instead.
/// RFC 5228 §4.4 — discard: silently drops the message.
#[test]
fn test_discard() {
    let script = compile(b"discard;").expect("compile failed");
    let msg = make_message(
        "bounce@example.com",
        "user@example.com",
        "Unwanted",
        "Drop me.",
    );
    let actions = evaluate(&script, &msg, "bounce@example.com", "user@example.com");
    // discard produces no delivery — the only action is Discard.
    assert_eq!(actions, vec![SieveAction::Discard]);
}

/// RFC 5228 §4.1 — reject: the reject extension (RFC 5429) refuses the message.
/// The require ["reject"] declaration is mandatory.
#[test]
fn test_reject() {
    let script = compile(b"require [\"reject\"]; reject \"No thanks\";").expect("compile failed");
    let msg = make_message(
        "unwanted@example.net",
        "user@example.com",
        "Rejected",
        "Body.",
    );
    let actions = evaluate(&script, &msg, "unwanted@example.net", "user@example.com");
    // The reason string must be preserved verbatim.
    assert_eq!(actions, vec![SieveAction::Reject("No thanks".to_string())]);
}

/// RFC 5228 §4.1 — implicit keep: a script that takes no explicit action
/// must result in Keep per RFC 5228 §2.10.2.
#[test]
fn test_implicit_keep_empty_script() {
    // Empty script — no statements at all.
    let script = compile(b"").expect("compile failed");
    let msg = make_message("a@example.com", "b@example.com", "Hi", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    assert_eq!(actions, vec![SieveAction::Keep]);
}

// ---------------------------------------------------------------------------
// RFC 5228 — conditional tests
// ---------------------------------------------------------------------------

/// RFC 5228 §5.9 — header test with :contains match type.
/// Script files into "INBOX.family" when From contains "@family.example".
/// Test with a matching message — expect FileInto.
#[test]
fn test_if_header_contains_match() {
    let script = compile(
        b"require [\"fileinto\"];\
          if header :contains \"From\" \"@family.example\" {\
              fileinto \"INBOX.family\";\
          }",
    )
    .expect("compile failed");

    // Message FROM contains the expected substring — branch taken.
    let msg = make_message(
        "mom@family.example",
        "me@example.com",
        "Dinner plans",
        "Coming over Saturday?",
    );
    let actions = evaluate(&script, &msg, "mom@family.example", "me@example.com");
    assert_eq!(
        actions,
        vec![SieveAction::FileInto("INBOX.family".to_string())]
    );
}

/// Same script as above but with a non-matching message — expect Keep.
#[test]
fn test_if_header_contains_no_match() {
    let script = compile(
        b"require [\"fileinto\"];\
          if header :contains \"From\" \"@family.example\" {\
              fileinto \"INBOX.family\";\
          }",
    )
    .expect("compile failed");

    // Message FROM does not contain "@family.example" — branch not taken.
    let msg = make_message(
        "stranger@other.example",
        "me@example.com",
        "Hello",
        "Do you know me?",
    );
    let actions = evaluate(&script, &msg, "stranger@other.example", "me@example.com");
    // No explicit disposition — implicit keep (RFC 5228 §2.10.2).
    assert_eq!(actions, vec![SieveAction::Keep]);
}

/// RFC 5228 §5.9 — header test with :is match type (exact case-insensitive match).
/// Tests the default comparator i;ascii-casemap.
#[test]
fn test_if_header_is_case_insensitive() {
    // Subject header value "hello world" matches key "Hello World" under
    // i;ascii-casemap (case-insensitive).
    let script = compile(
        b"require [\"fileinto\"];\
          if header :is \"Subject\" \"Hello World\" {\
              fileinto \"INBOX.greetings\";\
          }",
    )
    .expect("compile failed");

    let msg = make_message(
        "alice@example.com",
        "bob@example.com",
        "hello world",
        "Body.",
    );
    let actions = evaluate(&script, &msg, "alice@example.com", "bob@example.com");
    assert_eq!(
        actions,
        vec![SieveAction::FileInto("INBOX.greetings".to_string())]
    );
}

/// RFC 5228 §5.7 — exists test: true when the named header is present.
#[test]
fn test_exists_header_present() {
    let script = compile(
        b"require [\"fileinto\"];\
          if exists \"X-Spam-Flag\" {\
              fileinto \"Spam\";\
          }",
    )
    .expect("compile failed");

    let msg = make_message_with_header("X-Spam-Flag", "YES");
    let actions = evaluate(&script, &msg, "spammer@example.net", "user@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Spam".to_string())]);
}

/// RFC 5228 §5.7 — exists test: false when the named header is absent.
#[test]
fn test_exists_header_absent() {
    let script = compile(
        b"require [\"fileinto\"];\
          if exists \"X-Spam-Flag\" {\
              fileinto \"Spam\";\
          }",
    )
    .expect("compile failed");

    // Plain message without X-Spam-Flag.
    let msg = make_message("clean@example.com", "user@example.com", "Normal", "Hi.");
    let actions = evaluate(&script, &msg, "clean@example.com", "user@example.com");
    assert_eq!(actions, vec![SieveAction::Keep]);
}

/// RFC 5228 §5.11 — size test with :over.
/// A message longer than 10 bytes triggers FileInto.
#[test]
fn test_size_over_true() {
    let script = compile(
        b"require [\"fileinto\"];\
          if size :over 10 {\
              fileinto \"Big\";\
          }",
    )
    .expect("compile failed");

    // This message is well over 10 bytes.
    let msg = make_message(
        "a@example.com",
        "b@example.com",
        "Subject",
        "Body text here.",
    );
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Big".to_string())]);
}

/// RFC 5228 §5.11 — size test with :under.
/// A 1-byte message is under 10000 bytes — triggers FileInto.
#[test]
fn test_size_under_true() {
    let script = compile(
        b"require [\"fileinto\"];\
          if size :under 10000 {\
              fileinto \"Small\";\
          }",
    )
    .expect("compile failed");

    let msg = b"From: a@b.com\r\n\r\nx\r\n";
    let actions = evaluate(&script, msg, "a@b.com", "b@b.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Small".to_string())]);
}

// ---------------------------------------------------------------------------
// RFC 5228 — if / elsif / else chains
// ---------------------------------------------------------------------------

/// RFC 5228 §5 — elsif/else branching.
/// Script: if spam header → Spam; elsif list header → List; else → keep.
/// Test with spam header present.
#[test]
fn test_elsif_first_branch_taken() {
    let script = compile(
        b"require [\"fileinto\"];\
          if header :contains \"X-Spam-Score\" \"HIGH\" {\
              fileinto \"Junk\";\
          } elsif header :contains \"X-Mailing-List\" \"announcements\" {\
              fileinto \"Lists\";\
          } else {\
              keep;\
          }",
    )
    .expect("compile failed");

    let msg = make_message_with_header("X-Spam-Score", "HIGH");
    let actions = evaluate(&script, &msg, "spammer@example.net", "user@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Junk".to_string())]);
}

/// Same script — test with elsif condition matching (first branch not taken).
#[test]
fn test_elsif_second_branch_taken() {
    let script = compile(
        b"require [\"fileinto\"];\
          if header :contains \"X-Spam-Score\" \"HIGH\" {\
              fileinto \"Junk\";\
          } elsif header :contains \"X-Mailing-List\" \"announcements\" {\
              fileinto \"Lists\";\
          } else {\
              keep;\
          }",
    )
    .expect("compile failed");

    let msg = make_message_with_header("X-Mailing-List", "announcements@example.org");
    let actions = evaluate(&script, &msg, "list@example.org", "user@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Lists".to_string())]);
}

/// Same script — test with else branch (neither condition matches).
#[test]
fn test_elsif_else_branch_taken() {
    let script = compile(
        b"require [\"fileinto\"];\
          if header :contains \"X-Spam-Score\" \"HIGH\" {\
              fileinto \"Junk\";\
          } elsif header :contains \"X-Mailing-List\" \"announcements\" {\
              fileinto \"Lists\";\
          } else {\
              keep;\
          }",
    )
    .expect("compile failed");

    // Neither header present — falls through to else { keep }.
    let msg = make_message("normal@example.com", "user@example.com", "Hi", "Body.");
    let actions = evaluate(&script, &msg, "normal@example.com", "user@example.com");
    assert_eq!(actions, vec![SieveAction::Keep]);
}

// ---------------------------------------------------------------------------
// RFC 5228 — allof / anyof
// ---------------------------------------------------------------------------

/// RFC 5228 §5.4 — allof: both conditions must be true.
#[test]
fn test_allof_both_true() {
    let script = compile(
        b"require [\"fileinto\"];\
          if allof (header :contains \"From\" \"boss@example.com\",\
                    header :contains \"Subject\" \"URGENT\") {\
              fileinto \"Important\";\
          }",
    )
    .expect("compile failed");

    // Both conditions match.
    let msg = make_message(
        "boss@example.com",
        "employee@example.com",
        "URGENT: fix this",
        "Fix it.",
    );
    let actions = evaluate(&script, &msg, "boss@example.com", "employee@example.com");
    assert_eq!(
        actions,
        vec![SieveAction::FileInto("Important".to_string())]
    );
}

/// RFC 5228 §5.4 — allof: one false condition → no match.
#[test]
fn test_allof_one_false() {
    let script = compile(
        b"require [\"fileinto\"];\
          if allof (header :contains \"From\" \"boss@example.com\",\
                    header :contains \"Subject\" \"URGENT\") {\
              fileinto \"Important\";\
          }",
    )
    .expect("compile failed");

    // From matches but Subject does not → allof is false.
    let msg = make_message(
        "boss@example.com",
        "employee@example.com",
        "Just checking in",
        "How are you?",
    );
    let actions = evaluate(&script, &msg, "boss@example.com", "employee@example.com");
    assert_eq!(actions, vec![SieveAction::Keep]);
}

/// RFC 5228 §5.5 — anyof: either condition may be true.
#[test]
fn test_anyof_one_true() {
    let script = compile(
        b"require [\"fileinto\"];\
          if anyof (header :contains \"Subject\" \"win\",\
                    header :contains \"Subject\" \"prize\") {\
              fileinto \"Spam\";\
          }",
    )
    .expect("compile failed");

    // Only "win" matches.
    let msg = make_message(
        "lotto@example.net",
        "user@example.com",
        "You win a prize",
        "Congrats.",
    );
    let actions = evaluate(&script, &msg, "lotto@example.net", "user@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Spam".to_string())]);
}

// ---------------------------------------------------------------------------
// RFC 5228 — unsupported extension must fail at compile time
// ---------------------------------------------------------------------------

/// RFC 5228 §2.6 — require for an unknown extension must cause compile failure.
/// The vacation extension (RFC 5230) is not implemented in this crate.
#[test]
fn test_rfc5228_example_vacation_skip() {
    // "vacation" is not in the KNOWN extensions list; compile() must reject it.
    let result = compile(b"require [\"vacation\"]; vacation \"Out of office.\";");
    assert!(
        result.is_err(),
        "require of unsupported extension must fail at compile time"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("vacation") || err.message.contains("unsupported"),
        "error message should mention the extension or 'unsupported': {err}"
    );
}

/// A completely unknown extension also fails at compile time.
#[test]
fn test_unknown_extension_rejected() {
    let result = compile(b"require [\"nonexistent-extension\"];");
    assert!(result.is_err(), "unknown extension must fail compile");
}

// ---------------------------------------------------------------------------
// RFC 5229 — variables extension
// Based on RFC 5229 §5 examples and hand-traced expected outputs.
// ---------------------------------------------------------------------------

/// RFC 5229 §4 — set and use a variable in fileinto.
/// set "folder" "INBOX.Work" then fileinto "${folder}" → FileInto("INBOX.Work")
#[test]
fn test_variables_set_and_use() {
    let script = compile(
        b"require [\"variables\", \"fileinto\"];\
          set \"folder\" \"INBOX.Work\";\
          fileinto \"${folder}\";",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Work item", "Details.");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    assert_eq!(
        actions,
        vec![SieveAction::FileInto("INBOX.Work".to_string())]
    );
}

/// RFC 5229 §4 — :lower modifier converts value to lowercase.
/// set :lower "v" "HELLO" → variable holds "hello"
#[test]
fn test_variables_modifier_lower() {
    let script = compile(
        b"require [\"variables\", \"fileinto\"];\
          set :lower \"v\" \"UPPER\";\
          fileinto \"${v}\";",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("upper".to_string())]);
}

/// RFC 5229 §4 — :upper modifier converts value to uppercase.
#[test]
fn test_variables_modifier_upper() {
    let script = compile(
        b"require [\"variables\", \"fileinto\"];\
          set :upper \"v\" \"lower\";\
          fileinto \"${v}\";",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("LOWER".to_string())]);
}

/// RFC 5229 §4 — :length modifier replaces value with its character count.
/// set :length "n" "hello" → variable holds "5"
#[test]
fn test_variables_modifier_length() {
    let script = compile(
        b"require [\"variables\", \"fileinto\"];\
          set :length \"n\" \"hello\";\
          fileinto \"${n}\";",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    // "hello" has 5 characters.
    assert_eq!(actions, vec![SieveAction::FileInto("5".to_string())]);
}

/// RFC 5229 §3 — without require ["variables"], ${...} is treated as literal text.
/// RFC 5229 §3: "implementations MUST NOT perform variable substitution unless
/// the script requires the "variables" extension."
#[test]
fn test_no_variables_require_no_substitution() {
    // "${reason}" must be delivered verbatim when variables are not required.
    let script = compile(b"require [\"reject\"]; reject \"${reason}\";").expect("compile failed");
    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    assert_eq!(actions, vec![SieveAction::Reject("${reason}".to_string())]);
}

/// RFC 5229 §3 — variable names are case-insensitive.
/// set "MyVar" then reference as "${myvar}" must resolve correctly.
#[test]
fn test_variables_case_insensitive_name() {
    let script = compile(
        b"require [\"variables\", \"fileinto\"];\
          set \"MyVar\" \"hello\";\
          fileinto \"${myvar}\";",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("hello".to_string())]);
}

/// RFC 5229 §3 — variable substitution in test string arguments.
/// RFC 5229 §3: "Variable substitution is performed on all string arguments
/// ... in test commands."  A variable used as the key in a header test must
/// be expanded before matching.
#[test]
fn test_variables_expanded_in_header_test() {
    let script = compile(
        b"require [\"variables\", \"fileinto\"];\
          set \"keyword\" \"urgent\";\
          if header :contains \"Subject\" \"${keyword}\" {\
              fileinto \"Priority\";\
          }",
    )
    .expect("compile failed");

    // Subject contains "urgent" which matches the expanded variable value.
    let msg = make_message(
        "boss@example.com",
        "me@example.com",
        "urgent: meeting now",
        "Come now.",
    );
    let actions = evaluate(&script, &msg, "boss@example.com", "me@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Priority".to_string())]);

    // Subject does not contain "urgent" — variable still expands, no match.
    let msg2 = make_message(
        "friend@example.com",
        "me@example.com",
        "lunch plans",
        "See you at noon.",
    );
    let actions2 = evaluate(&script, &msg2, "friend@example.com", "me@example.com");
    assert_eq!(actions2, vec![SieveAction::Keep]);
}

/// RFC 5229 §3 — an undefined variable expands to the empty string.
#[test]
fn test_variables_undefined_expands_to_empty() {
    let script = compile(
        b"require [\"variables\", \"reject\"];\
          reject \"error: ${undefined_var}\";",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    // ${undefined_var} → "" so the reason is "error: "
    assert_eq!(actions, vec![SieveAction::Reject("error: ".to_string())]);
}

// ---------------------------------------------------------------------------
// RFC 5228 — stop command
// ---------------------------------------------------------------------------

/// RFC 5228 §4.5 — stop terminates execution; subsequent statements are not run.
/// Script: stop; fileinto "Never" — the fileinto must not execute.
/// When stop terminates without an explicit disposition, implicit keep applies.
#[test]
fn test_stop_terminates_execution() {
    let script = compile(
        b"require [\"fileinto\"];\
          stop;\
          fileinto \"Never\";",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    // stop with no prior disposition → implicit keep (RFC 5228 §2.10.2).
    assert_eq!(actions, vec![SieveAction::Keep]);
}

// ---------------------------------------------------------------------------
// RFC 5228 §5.1 — address test with multi-address headers (issue 10p.33)
// ---------------------------------------------------------------------------

/// RFC 5228 §5.1 — when a header contains multiple comma-separated addresses,
/// each one is tested independently.  A match on any single address suffices.
///
/// Expected output derived by hand: the To header contains two addresses;
/// the address test with :is must match the second one individually.
#[test]
fn test_address_multi_address_header_match() {
    let script = compile(
        b"require [\"fileinto\"];\
          if address :is \"To\" \"bob@example.com\" {\
              fileinto \"Found\";\
          }",
    )
    .expect("compile failed");

    // To header has two addresses; "bob@example.com" is the second.
    let msg = b"From: alice@example.com\r\nTo: carol@example.com, bob@example.com\r\nSubject: Hi\r\n\r\nBody.\r\n";
    let actions = evaluate(&script, msg, "alice@example.com", "bob@example.com");
    assert_eq!(actions, vec![SieveAction::FileInto("Found".to_string())]);
}

/// Same setup but the address under test is NOT in the To header — expect Keep.
#[test]
fn test_address_multi_address_header_no_match() {
    let script = compile(
        b"require [\"fileinto\"];\
          if address :is \"To\" \"unknown@example.com\" {\
              fileinto \"Found\";\
          }",
    )
    .expect("compile failed");

    let msg = b"From: alice@example.com\r\nTo: carol@example.com, bob@example.com\r\nSubject: Hi\r\n\r\nBody.\r\n";
    let actions = evaluate(&script, msg, "alice@example.com", "bob@example.com");
    assert_eq!(actions, vec![SieveAction::Keep]);
}

// ---------------------------------------------------------------------------
// RFC 5228 §5.7 — exists with empty argument list (issue 10p.35)
// ---------------------------------------------------------------------------

/// exists with an empty string-list must return false, not vacuously true.
///
/// RFC 5228 §5.7: exists tests that every listed header is present.
/// With no headers listed there is nothing to assert present — the result
/// is false, not a vacuous truth.  Expected output hand-traced: no match
/// → implicit keep.
#[test]
fn test_exists_empty_list_is_false() {
    let script = compile(
        b"require [\"fileinto\"];\
          if exists [] {\
              fileinto \"Matched\";\
          }",
    )
    .expect("compile failed");

    let msg = make_message("a@example.com", "b@example.com", "Test", ".");
    let actions = evaluate(&script, &msg, "a@example.com", "b@example.com");
    // Empty list → false → branch not taken → implicit keep.
    assert_eq!(actions, vec![SieveAction::Keep]);
}

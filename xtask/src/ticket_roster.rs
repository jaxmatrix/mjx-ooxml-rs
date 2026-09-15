//! The tickets an `xtask` gate accepts as real: the renderer-completion epic's roster (MJXOFF-295) and the tickets filed beside it.
//!
//! Shared by `#[path]` include between `xtask/tests/feature_checklist.rs` and `xtask/tests/deferrals_cite_tickets.rs`, so the two gates cannot disagree about which ids exist.
#![allow(dead_code)]

// The renderer-completion epic's (MJXOFF-295) RC label to ticket pairs; RC24 also owns sub-ticket MJXOFF-211.
pub(crate) const TICKET_ROSTER: [(&str, &str); 48] = [
    ("RC00", "MJXOFF-296"),
    ("RC01", "MJXOFF-297"),
    ("RC47", "MJXOFF-298"),
    ("RC02", "MJXOFF-299"),
    ("RC03", "MJXOFF-300"),
    ("RC04", "MJXOFF-243"),
    ("RC05", "MJXOFF-301"),
    ("RC06", "MJXOFF-302"),
    ("RC07", "MJXOFF-303"),
    ("RC08", "MJXOFF-304"),
    ("RC09", "MJXOFF-255"),
    ("RC10", "MJXOFF-305"),
    ("RC11", "MJXOFF-306"),
    ("RC12", "MJXOFF-307"),
    ("RC13", "MJXOFF-308"),
    ("RC14", "MJXOFF-309"),
    ("RC15", "MJXOFF-310"),
    ("RC16", "MJXOFF-311"),
    ("RC17", "MJXOFF-312"),
    ("RC18", "MJXOFF-313"),
    ("RC20", "MJXOFF-314"),
    ("RC21", "MJXOFF-315"),
    ("RC22", "MJXOFF-316"),
    ("RC23", "MJXOFF-317"),
    ("RC24", "MJXOFF-318"),
    ("RC24", "MJXOFF-211"),
    ("RC25", "MJXOFF-319"),
    ("RC26", "MJXOFF-320"),
    ("RC27", "MJXOFF-259"),
    ("RC28", "MJXOFF-321"),
    ("RC29", "MJXOFF-322"),
    ("RC30", "MJXOFF-323"),
    ("RC31", "MJXOFF-324"),
    ("RC32", "MJXOFF-325"),
    ("RC33", "MJXOFF-326"),
    ("RC34", "MJXOFF-327"),
    ("RC35", "MJXOFF-328"),
    ("RC36", "MJXOFF-329"),
    ("RC37", "MJXOFF-330"),
    ("RC38", "MJXOFF-331"),
    ("RC39", "MJXOFF-332"),
    ("RC40", "MJXOFF-333"),
    ("RC41", "MJXOFF-334"),
    ("RC42", "MJXOFF-335"),
    ("RC43", "MJXOFF-258"),
    ("RC44", "MJXOFF-336"),
    ("RC45", "MJXOFF-337"),
    ("RC46", "MJXOFF-338"),
];

// Backlog tickets filed beside the epic's roster that a deferral may still be owned by.
pub(crate) const TICKETS_BESIDE_THE_ROSTER: [&str; 10] = [
    "MJXOFF-348",
    "MJXOFF-349",
    "MJXOFF-350",
    "MJXOFF-351",
    "MJXOFF-352",
    "MJXOFF-353",
    "MJXOFF-354",
    "MJXOFF-355",
    "MJXOFF-356",
    "MJXOFF-357",
];

// Tickets of the roster whose work is merged, so no deferral or checklist row may name one as its owner, each with why.
pub(crate) const CLOSED_TICKETS: [(&str, &str); 3] = [
    ("MJXOFF-296", "RC00, the feature checklist, is merged"),
    (
        "MJXOFF-297",
        "RC01, the ledger and checklist reconciliation, is merged",
    ),
    (
        "MJXOFF-298",
        "RC47, the deferral and completion-claim gates, closes with its audit fixes",
    ),
];

// Why `candidate` is closed, or `None` when it is not.
pub(crate) fn closed_reason(candidate: &str) -> Option<&'static str> {
    CLOSED_TICKETS
        .iter()
        .find(|(ticket, _)| *ticket == candidate)
        .map(|(_, reason)| *reason)
}

// Whether `candidate` is a ticket of the epic's roster.
pub(crate) fn is_roster_ticket(candidate: &str) -> bool {
    TICKET_ROSTER.iter().any(|(_, ticket)| *ticket == candidate)
}

// Whether `candidate` is any ticket a gate accepts as real.
pub(crate) fn is_known_ticket(candidate: &str) -> bool {
    is_roster_ticket(candidate) || TICKETS_BESIDE_THE_ROSTER.contains(&candidate)
}

// Whether `candidate` is a real ticket that is still open, so it may own a deferral or a row.
pub(crate) fn is_open_ticket(candidate: &str) -> bool {
    is_known_ticket(candidate) && closed_reason(candidate).is_none()
}

//! The shared-workbook feature: who has the workbook open, and what each editing session changed.
//!
//! Three parts and twenty-two complex types (`sml.xsd:1860–2117`), of which this file reads the two
//! index parts and **not one revision**. The reason is narrower than the pivot cluster's: a revision
//! log is a list of *undo records* — `x:rcc` a changed cell, `x:rrc` a deleted range, `x:rm` a move
//! — and a model of them that could not replay them would be a model of nothing. Replaying them
//! means recomputing every cell they touch, which is again the calculation model this library does
//! not have.
//!
//! What a caller genuinely needs is upstream of all that: **is this workbook shared, and if so who
//! by**. Editing a shared workbook without understanding its revision history is how a consumer
//! corrupts one, so knowing to be careful is worth more than a typed `rcc`. That is what
//! [`RevisionHeadersIdentity`] and [`SharedWorkbookUsersIdentity`] answer.
//!
//! # This library never writes a revision
//!
//! Editing a cell here does **not** append an `x:rcc` to the revision log, and nothing here
//! pretends otherwise. A workbook saved by this library after an edit therefore has a revision
//! history that no longer describes it — which is exactly why
//! `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md` says so in the gap table rather than
//! leaving a caller to find out. The logs' bytes survive untouched; their *meaning* is the file's
//! and not this library's to maintain.

use mjx_ooxml_core::{Interner, RawElement};

use crate::error::SmlError;

use super::{
    is_sml, optional_boolean, optional_text, optional_unsigned, relationship_id, required_text,
    sml_children,
};

/// One editing session recorded in the revision headers part (`x:header`, `CT_RevisionHeader`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionSession {
    /// `@guid` (`use="required"`) — this session's identifier.
    pub guid: String,
    /// `@dateTime` (`use="required"`) — as the file wrote it. An `xsd:dateTime` string, kept as
    /// text: this crate has no date type and inventing one for a value it never computes with
    /// would be a type nobody needs.
    pub date_time: String,
    /// `@userName` (`use="required"`) — who edited in this session.
    pub user_name: String,
    /// `@r:id` (`use="required"`) — the explicit relationship to this session's revision log part.
    pub relationship_id: Option<String>,
}

/// The revision headers part (`x:headers`, `CT_RevisionHeaders`), identified — the index of a shared
/// workbook's editing sessions.
///
/// **Its mere presence is the answer to "is this workbook shared".** §12.3.16 permits at most one
/// such part and makes it the target of an implicit relationship from the workbook, so a workbook
/// that relates to one is in shared mode and one that does not is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionHeadersIdentity {
    /// `@guid` (`use="required"`) — the shared-workbook identifier every log agrees on.
    pub guid: String,
    /// `@lastGuid` — the last session's identifier, when the file states one.
    pub last_guid: Option<String>,
    /// `@shared`, or the schema default `true`.
    pub shared: bool,
    /// `@trackRevisions`, or the schema default `true`.
    pub tracks_revisions: bool,
    /// `@preserveHistory`, or the schema default 30 — how many days of history the producer keeps.
    /// **Reported, never enforced:** nothing here expires a log.
    pub preserve_history_days: u32,
    /// The sessions, in document order, which is the order they happened in.
    pub sessions: Vec<RevisionSession>,
}

impl RevisionHeadersIdentity {
    /// Reads a revision headers part's root element, or `Ok(None)` when `root` is not an
    /// `x:headers`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `use="required"` attribute of the part or of one of its sessions is
    /// absent or will not decode.
    pub fn read_root(
        root: &RawElement,
        interner: &Interner,
        reference_prefix: Option<&str>,
    ) -> Result<Option<Self>, SmlError> {
        if !is_sml(root, interner, "headers") {
            return Ok(None);
        }
        let mut sessions = Vec::new();
        for header in sml_children(root, interner, "header") {
            sessions.push(RevisionSession {
                guid: required_text(&header.attributes, interner, "guid")?,
                date_time: required_text(&header.attributes, interner, "dateTime")?,
                user_name: required_text(&header.attributes, interner, "userName")?,
                relationship_id: relationship_id(header, interner, reference_prefix)?,
            });
        }
        Ok(Some(Self {
            guid: required_text(&root.attributes, interner, "guid")?,
            last_guid: optional_text(&root.attributes, interner, "lastGuid")?,
            shared: optional_boolean(&root.attributes, interner, "shared", true)?,
            tracks_revisions: optional_boolean(&root.attributes, interner, "trackRevisions", true)?,
            preserve_history_days: optional_unsigned(
                &root.attributes,
                interner,
                "preserveHistory",
            )?
            .unwrap_or(30),
            sessions,
        }))
    }
}

/// One user sharing the workbook (`x:userInfo`, `CT_SharedUser`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedWorkbookUser {
    /// `@id` (`use="required"`) — `xsd:int`, so it may legitimately be negative, and it is reported
    /// as the file wrote it rather than clamped.
    pub id: i32,
    /// `@name` (`use="required"`).
    pub name: String,
    /// `@guid` (`use="required"`).
    pub guid: String,
    /// `@dateTime` (`use="required"`) — as text, for the reason [`RevisionSession::date_time`]
    /// gives.
    pub date_time: String,
}

/// The shared-workbook user data part (`x:users`, `CT_Users`), identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedWorkbookUsersIdentity {
    /// `@count` — how many users the file *says* there are. A count the file states, reported
    /// without being checked against [`users`](Self::users), exactly as `sst/@count` is.
    pub stated_count: Option<u32>,
    /// The users, in document order. `CT_Users` caps the list at `maxOccurs="256"`; nothing here
    /// enforces that, because a file exceeding it is one this library still reads back byte for
    /// byte.
    pub users: Vec<SharedWorkbookUser>,
}

impl SharedWorkbookUsersIdentity {
    /// Reads a user data part's root element, or `Ok(None)` when `root` is not an `x:users`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `use="required"` attribute of one of the entries is absent or will
    /// not decode.
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        if !is_sml(root, interner, "users") {
            return Ok(None);
        }
        let mut users = Vec::new();
        for info in sml_children(root, interner, "userInfo") {
            let id = required_text(&info.attributes, interner, "id")?;
            users.push(SharedWorkbookUser {
                id: id.trim().parse::<i32>().map_err(|error| {
                    mjx_ooxml_core::FromXmlError::from(
                        mjx_ooxml_core::AttributeError::InvalidValue {
                            attribute: "id",
                            detail: error.to_string(),
                        },
                    )
                })?,
                name: required_text(&info.attributes, interner, "name")?,
                guid: required_text(&info.attributes, interner, "guid")?,
                date_time: required_text(&info.attributes, interner, "dateTime")?,
            });
        }
        Ok(Some(Self {
            stated_count: optional_unsigned(&root.attributes, interner, "count")?,
            users,
        }))
    }
}

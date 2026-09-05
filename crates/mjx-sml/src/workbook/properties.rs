//! What the workbook *is*: `workbookPr`, `fileVersion`, `fileSharing`, `workbookProtection`,
//! `fileRecoveryPr` and `oleSize` — six of `CT_Workbook`'s nineteen slots, every one of them an
//! attribute bag.
//!
//! # `date1904` is exposed, never applied
//!
//! `workbookPr/@date1904` selects which epoch a date-serial number counts from: 1899-12-30 (the
//! 1900 system, and the default) or 1904-01-01 (the Macintosh system). It changes what **every**
//! date in the workbook means, and it is therefore the one attribute here with teeth.
//!
//! [`WorkbookProperties::uses_1904_date_system`] reports it and nothing in this workspace acts on
//! it, because nothing in this workspace does date arithmetic: a cell's value is the number the file
//! wrote (`crate::cells`), and turning that number into a calendar date is the caller's decision to
//! make with the epoch this accessor hands them. A library that quietly converted would have to
//! convert *back* on write, and a round trip through two conversions is exactly the fidelity risk
//! this project exists to avoid.
//!
//! # Passwords are opaque, never recomputed
//!
//! `CT_WorkbookProtection` and `CT_FileSharing` carry legacy 16-bit password hashes
//! (`ST_UnsignedShortHex`) beside modern `algorithmName`/`hashValue`/`saltValue`/`spinCount` triples.
//! Every one is read and written as the exact wire text. Nothing here computes, verifies or clears a
//! hash — the same position `mjx_docx`'s `DocumentProtection` states for `w:documentProtection`, for
//! the same reason: a structural editor that changed a lock's *value* while claiming to preserve the
//! file would be lying about both.

use mjx_ooxml_core::{Enumeration, Number, Text};
use mjx_ooxml_types::spreadsheetml::{ObjectDisplay, UpdateLinksBehavior};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRange;

use super::leaf::attribute_bag;

attribute_bag! {
    /// `x:fileVersion` (`CT_FileVersion`, `sml.xsd:4125`) — which application last wrote the
    /// workbook, and at which build.
    ///
    /// Every attribute is `xsd:string` in the schema, including `lastEdited`/`lowestEdited`/
    /// `rupBuild`, which look like numbers and are not: Excel writes `"4"` and `"20952"` there, but
    /// nothing in the schema stops a producer writing anything else, so they are carried as the
    /// text the file holds. `sample.xlsx` writes `<fileVersion appName="Calc"/>` and nothing more.
    #[xml(attribute(local = "appName", codec = Text, accessor = application_name))]
    #[xml(attribute(local = "lastEdited", codec = Text, accessor = last_edited_version))]
    #[xml(attribute(local = "lowestEdited", codec = Text, accessor = lowest_edited_version))]
    #[xml(attribute(local = "rupBuild", codec = Text, accessor = build_number))]
    #[xml(attribute(local = "codeName", codec = Text, accessor = code_name))]
    FileVersion, "fileVersion"
}

attribute_bag! {
    /// `x:workbookPr` (`CT_WorkbookPr`, `sml.xsd:4229`) — the workbook's own settings: the date
    /// system, what a consumer draws, and a dozen producer preferences.
    ///
    /// **`sample.xlsx`'s is the reason this type must not rebuild its attribute vector.** It reads
    /// `<workbookPr backupFile="false" showObjects="all" dateCompatibility="false"/>`, and
    /// `dateCompatibility` is **not declared** by the Transitional `sml.xsd` — it is LibreOffice's.
    /// It survives because the `attribute_bag!` macro every type in this cluster is declared by
    /// keeps the vector the file wrote and never assembles a new one; `crates/mjx-schema-gate/src/tolerances.rs` records the deviation
    /// so the schema gate reports it as tolerated rather than failing.
    #[xml(attribute(local = "date1904", codec = OnOff, accessor = uses_1904_date_system, default = false))]
    #[xml(attribute(local = "showObjects", codec = Enumeration<ObjectDisplay>, accessor = object_display, default = ObjectDisplay::All))]
    #[xml(attribute(local = "showBorderUnselectedTables", codec = OnOff, accessor = show_borders_on_unselected_tables, default = true))]
    #[xml(attribute(local = "filterPrivacy", codec = OnOff, accessor = filter_privacy, default = false))]
    #[xml(attribute(local = "promptedSolutions", codec = OnOff, accessor = prompted_for_solutions, default = false))]
    #[xml(attribute(local = "showInkAnnotation", codec = OnOff, accessor = show_ink_annotations, default = true))]
    #[xml(attribute(local = "backupFile", codec = OnOff, accessor = create_backup_file, default = false))]
    #[xml(attribute(local = "saveExternalLinkValues", codec = OnOff, accessor = save_external_link_values, default = true))]
    #[xml(attribute(local = "updateLinks", codec = Enumeration<UpdateLinksBehavior>, accessor = update_links_behavior, default = UpdateLinksBehavior::UserSet))]
    #[xml(attribute(local = "codeName", codec = Text, accessor = code_name))]
    #[xml(attribute(local = "hidePivotFieldList", codec = OnOff, accessor = hide_pivot_field_list, default = false))]
    #[xml(attribute(local = "showPivotChartFilter", codec = OnOff, accessor = show_pivot_chart_filter, default = false))]
    #[xml(attribute(local = "allowRefreshQuery", codec = OnOff, accessor = allow_refresh_query, default = false))]
    #[xml(attribute(local = "publishItems", codec = OnOff, accessor = publish_items, default = false))]
    #[xml(attribute(local = "checkCompatibility", codec = OnOff, accessor = check_compatibility_on_save, default = false))]
    #[xml(attribute(local = "autoCompressPictures", codec = OnOff, accessor = auto_compress_pictures, default = true))]
    #[xml(attribute(local = "refreshAllConnections", codec = OnOff, accessor = refresh_all_connections_on_open, default = false))]
    #[xml(attribute(local = "defaultThemeVersion", codec = Number<u32>, accessor = default_theme_version))]
    WorkbookProperties, "workbookPr"
}

attribute_bag! {
    /// `x:fileSharing` (`CT_FileSharing`, `sml.xsd:4359`) — the write-reservation password and the
    /// read-only recommendation a consumer shows before opening.
    ///
    /// `reservationPassword` is the legacy `ST_UnsignedShortHex` form and
    /// `algorithmName`/`hashValue`/`saltValue`/`spinCount` the modern one; both are carried as the
    /// exact wire text. See this module's own documentation for why neither is ever recomputed.
    #[xml(attribute(local = "readOnlyRecommended", codec = OnOff, accessor = read_only_recommended, default = false))]
    #[xml(attribute(local = "userName", codec = Text, accessor = user_name))]
    #[xml(attribute(local = "reservationPassword", codec = Text, accessor = legacy_reservation_password_hash))]
    #[xml(attribute(local = "algorithmName", codec = Text, accessor = password_algorithm_name))]
    #[xml(attribute(local = "hashValue", codec = Text, accessor = password_hash_value))]
    #[xml(attribute(local = "saltValue", codec = Text, accessor = password_salt_value))]
    #[xml(attribute(local = "spinCount", codec = Number<u32>, accessor = password_spin_count))]
    FileSharing, "fileSharing"
}

attribute_bag! {
    /// `x:workbookProtection` (`CT_WorkbookProtection`, `sml.xsd:4371`) — whether the sheet
    /// structure and the window layout may be changed, and the two password families that guard
    /// them.
    ///
    /// `sample.xlsx` writes `<workbookProtection/>`: present, with every attribute absent, so every
    /// getter here answers with the schema's own default and the element still writes back as the
    /// empty tag it was.
    #[xml(attribute(local = "workbookPassword", codec = Text, accessor = legacy_workbook_password_hash))]
    #[xml(attribute(local = "workbookPasswordCharacterSet", codec = Text, accessor = workbook_password_character_set))]
    #[xml(attribute(local = "revisionsPassword", codec = Text, accessor = legacy_revisions_password_hash))]
    #[xml(attribute(local = "revisionsPasswordCharacterSet", codec = Text, accessor = revisions_password_character_set))]
    #[xml(attribute(local = "lockStructure", codec = OnOff, accessor = lock_structure, default = false))]
    #[xml(attribute(local = "lockWindows", codec = OnOff, accessor = lock_windows, default = false))]
    #[xml(attribute(local = "lockRevision", codec = OnOff, accessor = lock_revision_tracking, default = false))]
    #[xml(attribute(local = "revisionsAlgorithmName", codec = Text, accessor = revisions_password_algorithm_name))]
    #[xml(attribute(local = "revisionsHashValue", codec = Text, accessor = revisions_password_hash_value))]
    #[xml(attribute(local = "revisionsSaltValue", codec = Text, accessor = revisions_password_salt_value))]
    #[xml(attribute(local = "revisionsSpinCount", codec = Number<u32>, accessor = revisions_password_spin_count))]
    #[xml(attribute(local = "workbookAlgorithmName", codec = Text, accessor = workbook_password_algorithm_name))]
    #[xml(attribute(local = "workbookHashValue", codec = Text, accessor = workbook_password_hash_value))]
    #[xml(attribute(local = "workbookSaltValue", codec = Text, accessor = workbook_password_salt_value))]
    #[xml(attribute(local = "workbookSpinCount", codec = Number<u32>, accessor = workbook_password_spin_count))]
    WorkbookProtection, "workbookProtection"
}

attribute_bag! {
    /// `x:fileRecoveryPr` (`CT_FileRecoveryPr`, `sml.xsd:4278`) — what a consumer that crashed last
    /// time should do on the next open.
    ///
    /// The **one repeatable slot** in `CT_Workbook`'s nineteen: the schema declares it
    /// `maxOccurs="unbounded"`, so [`super::WorkbookPart`] holds a list of these and never assumes
    /// there is at most one.
    #[xml(attribute(local = "autoRecover", codec = OnOff, accessor = auto_recover, default = true))]
    #[xml(attribute(local = "crashSave", codec = OnOff, accessor = saved_after_a_crash, default = false))]
    #[xml(attribute(local = "dataExtractLoad", codec = OnOff, accessor = data_extract_load, default = false))]
    #[xml(attribute(local = "repairLoad", codec = OnOff, accessor = repair_on_load, default = false))]
    FileRecoveryProperties, "fileRecoveryPr"
}

attribute_bag! {
    /// `x:oleSize` (`CT_OleSize`, `sml.xsd:4368`) — the cell range an OLE container shows when the
    /// workbook is embedded in another document rather than opened on its own.
    ///
    /// `@ref` is `ST_Ref`, which is D03's [`CellRange`] and not a second address type: the range is
    /// decoded through the same parser `sqref`, `spans` and every worksheet dimension go through, so
    /// a `$`-anchored or whole-column form is understood here exactly as it is there.
    #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
    EmbeddedObjectSize, "oleSize"
}

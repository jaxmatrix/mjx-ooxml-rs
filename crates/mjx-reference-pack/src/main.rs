//! The reference pack's command line: generate the artefacts, and run the preliminary pass.
//!
//! ```sh
//! cargo run -p mjx-reference-pack -- generate     target/reference-pack
//! cargo run -p mjx-reference-pack -- preliminary  target/reference-pack
//! cargo run -p mjx-reference-pack -- items
//! ```
//!
//! **`preliminary` is a report and not a gate.** It prints one row per plate and exits zero on a
//! disagreement, deliberately: a LibreOffice difference is a thing to look at before a Windows
//! morning, and making it a build failure would be the first step towards editing correct code
//! until it matches a reference that is explicitly not authoritative.

use std::path::PathBuf;
use std::process::ExitCode;

use mjx_reference_pack::pack::{generate, preliminary_pass, report};
use mjx_reference_pack::plates::PresetDeck;
use mjx_reference_pack::{ARTEFACTS, OFFICE_EXPORT_DIRECTORY, THE_SITTING};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let command = arguments.first().map(String::as_str);
    let directory = arguments
        .get(1)
        .map_or_else(|| PathBuf::from("target/reference-pack"), PathBuf::from);

    match command {
        Some("generate") => match generate(&directory) {
            Ok(written) => {
                for path in written {
                    println!("wrote {}", path.display());
                }
                println!(
                    "\nOpen each in the real Microsoft application, export it to PDF, and put the \
                     PDFs in {OFFICE_EXPORT_DIRECTORY}. INSTRUCTIONS.md beside the artefacts says \
                     what not to do."
                );
                ExitCode::SUCCESS
            }
            Err(reason) => {
                eprintln!("{reason}");
                ExitCode::FAILURE
            }
        },
        Some("preliminary") => {
            let mut failed = false;
            for which in PresetDeck::ALL {
                println!("== {} ==", which.file_name());
                match preliminary_pass(&directory, which) {
                    Ok(pass) => print!("{}", report(&pass.baselines)),
                    Err(reason) => {
                        eprintln!("{reason}");
                        failed = true;
                    }
                }
            }
            // The pass itself failing to run is a failure; a plate disagreeing is not.
            if failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        // ⚠ Takes no directory and no provider. See `pack::office_pass`: a call that took either
        // would let a LibreOffice PDF be pointed at it and recorded as Office's, which is the trap
        // this whole crate is built against.
        Some("ingest") => match mjx_reference_pack::pack::office_pass() {
            Ok(rows) => {
                print!("{}", report(&rows));
                println!(
                    "\nRead from {OFFICE_EXPORT_DIRECTORY}, which is the only directory Office \
                     exports live in."
                );
                ExitCode::SUCCESS
            }
            Err(reason) => {
                eprintln!("{reason}");
                ExitCode::FAILURE
            }
        },
        Some("items") => {
            println!("The Windows sitting answers {} items:\n", THE_SITTING.len());
            for item in THE_SITTING {
                println!("{:<26} {}", item.key, item.artefact);
                println!("    {}\n", item.question);
            }
            println!("Artefacts: {}", ARTEFACTS.join(", "));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: mjx-reference-pack <generate|preliminary|ingest|items> [directory]\n\n\
                 generate     author the four artefacts and their instructions\n\
                 preliminary  convert them with LibreOffice and report one row per plate\n\
                 ingest       read whatever Microsoft Office exported, out of the one directory \
                 Office exports live in (no arguments, on purpose)\n\
                 items        what the sitting answers"
            );
            ExitCode::FAILURE
        }
    }
}

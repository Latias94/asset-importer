//! List supported import/export formats and version info

mod common;

use asset_importer::{get_import_extensions, version};

#[cfg(feature = "export")]
use asset_importer::get_export_formats;

fn main() {
    common::init_logging_from_env();
    println!("Assimp version: {}", version::assimp_version());

    let exts = get_import_extensions();
    println!("Import extensions ({}):", exts.len());
    for e in exts.iter().take(80) {
        print!("{} ", e);
    }
    println!();

    #[cfg(feature = "export")]
    {
        let formats = get_export_formats();
        println!("Export formats ({}):", formats.len());
        for f in formats.iter().take(40) {
            println!("  {} -> .{} ({})", f.id, f.file_extension, f.description);
        }
    }

    #[cfg(not(feature = "export"))]
    println!("(Enable `export` feature to list export formats)");

    common::shutdown_logging();
}

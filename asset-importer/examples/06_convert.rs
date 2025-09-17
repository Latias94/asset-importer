//! Convert a model to another format based on output extension

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

#[cfg(feature = "export")]
use asset_importer::{Scene, exporter::ExportBuilder, get_export_formats};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <input_model> <output_file>",
            args.first().unwrap_or(&"06_convert".to_string())
        );
        std::process::exit(1);
    }
    let _input = std::path::Path::new(&args[1]);
    let _output = std::path::Path::new(&args[2]);

    let result = {
        #[cfg(not(feature = "export"))]
        {
            eprintln!("This example requires the 'export' feature. Re-run with: --features export");
            Err("Export feature not enabled".into())
        }

        #[cfg(feature = "export")]
        {
            let scene: Scene = common::import_scene(input, PostProcessSteps::TRIANGULATE)?;
            let ext = output
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let id = match map_extension_to_id(&ext) {
                Some(id) => id,
                None => {
                    eprintln!("Unknown export extension '.{}'", ext);
                    return Err(format!("Unknown export extension '.{}'", ext).into());
                }
            };
            ExportBuilder::new(id).export_to_file(&scene, output)?;
            println!("Wrote {}", output.display());
            Ok(())
        }
    };

    common::shutdown_logging();

    #[cfg(not(feature = "export"))]
    if result.is_err() {
        std::process::exit(1);
    }

    result
}

#[cfg(feature = "export")]
fn map_extension_to_id(ext: &str) -> Option<String> {
    for f in get_export_formats() {
        if f.file_extension.eq_ignore_ascii_case(ext) {
            return Some(f.id);
        }
    }
    None
}

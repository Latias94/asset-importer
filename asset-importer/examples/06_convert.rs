//! Convert a model to another format based on output extension

mod common;

use std::error::Error;

use asset_importer::postprocess::PostProcessSteps;
#[cfg(feature = "export")]
use asset_importer::{exporter::ExportBuilder, get_export_formats, Scene};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <input_model> <output_file>",
            args.get(0).unwrap_or(&"06_convert".to_string())
        );
        std::process::exit(1);
    }
    let input = std::path::Path::new(&args[1]);
    let output = std::path::Path::new(&args[2]);

    #[cfg(not(feature = "export"))]
    {
        eprintln!("This example requires the 'export' feature. Re-run with: --features export");
        std::process::exit(1);
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
                std::process::exit(2);
            }
        };
        ExportBuilder::new(id).export_to_file(&scene, output)?;
        println!("Wrote {}", output.display());
    }

    common::shutdown_logging();
    Ok(())
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

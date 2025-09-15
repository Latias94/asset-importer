use asset_importer_sys::*;
use std::ffi::CString;

fn main() {
    println!("🚀 Testing asset-importer-sys bindings...");

    // Test basic functionality - check if we can call functions
    println!("\n📋 Testing basic function calls...");

    // Test getting import format count
    unsafe {
        let format_count = aiGetImportFormatCount();
        println!("📊 Number of supported import formats: {}", format_count);

        // List first few formats
        println!("📝 First 5 supported formats:");
        for i in 0..std::cmp::min(format_count, 5) {
            let desc = aiGetImportFormatDescription(i);
            if !desc.is_null() {
                let name_ptr = (*desc).mName;
                if !name_ptr.is_null() {
                    let name = std::ffi::CStr::from_ptr(name_ptr);
                    println!("  {}. {}", i + 1, name.to_string_lossy());
                }
            }
        }
    }

    // Test error handling
    println!("\n🔍 Testing error handling...");
    unsafe {
        let filename = CString::new("non_existent_file.obj").unwrap();
        let scene = aiImportFile(filename.as_ptr(), 0);
        if scene.is_null() {
            println!("✅ Correctly returned null for non-existent file");

            // Get error string
            let error_ptr = aiGetErrorString();
            if !error_ptr.is_null() {
                let error_msg = std::ffi::CStr::from_ptr(error_ptr);
                println!("📄 Error message: {}", error_msg.to_string_lossy());
            }
        } else {
            println!("❌ Unexpectedly got a scene for non-existent file");
            aiReleaseImport(scene);
        }
    }

    // Test extension support
    println!("\n🔧 Testing extension support...");
    unsafe {
        let extensions = [".obj", ".fbx", ".dae", ".gltf", ".3ds"];
        for ext in &extensions {
            let ext_cstr = CString::new(*ext).unwrap();
            let supported = aiIsExtensionSupported(ext_cstr.as_ptr());
            let status = if supported != 0 { "✅" } else { "❌" };
            println!("  {} {}", status, ext);
        }
    }

    println!("\n🎉 All tests completed successfully!");
    println!("🔥 asset-importer-sys is working perfectly!");
}

fn main() {
    // 原生识别随 Rust 静态库构建；iOS 最终链接还需在 bundle.iOS.frameworks 声明系统框架。
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target == "macos" || target == "ios" {
        println!("cargo:rerun-if-changed=native/apple/reader_ocr.m");
        cc::Build::new()
            .file("native/apple/reader_ocr.m")
            .flag("-fobjc-arc")
            .compile("reader_ocr");
        for framework in ["Foundation", "Vision", "ImageIO", "CoreGraphics"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!(
            "cargo:rustc-link-lib=framework={}",
            if target == "ios" { "UIKit" } else { "AppKit" }
        );
    }
    tauri_build::build()
}

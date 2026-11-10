//! Build script to increase stack size for Servo's SpiderMonkey TLS
//!
//! Servo's SpiderMonkey integration uses large thread-local storage that
//! exceeds the default stack size. This build script sets the main thread's
//! stack to 32MB. The Servo engine thread also explicitly sets 16MB stack.

fn main() {
    // Apply to all builds (both debug and release)
    // Platform-specific stack size configuration
    #[cfg(target_os = "linux")]
    {
        // Set main thread stack size to 32MB
        // Servo's SpiderMonkey needs ~8MB+ for TLS
        println!("cargo:rustc-link-arg=-Wl,-z,stack-size=33554432");
    }

    #[cfg(target_os = "macos")]
    {
        // macOS uses a different linker flag
        println!("cargo:rustc-link-arg=-Wl,-stack_size,33554432");
    }

    #[cfg(target_os = "windows")]
    {
        // Windows uses a different mechanism
        println!("cargo:rustc-link-arg=/STACK:33554432");
    }

    // Rebuild if this script changes
    println!("cargo:rerun-if-changed=build.rs");
}

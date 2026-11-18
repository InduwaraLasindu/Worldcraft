fn main() {
    println!("cargo:rerun-if-changed=../../tools/ci/check_fsr_abi.cpp");
    println!("cargo:rerun-if-changed=vendor/fsr-api");
    println!("cargo:rerun-if-env-changed=WORLDCRAFT_CHECK_FSR_ABI");
    println!("cargo:rerun-if-env-changed=VULKAN_SDK");
    if std::env::var_os("WORLDCRAFT_CHECK_FSR_ABI").is_some() {
        let sdk = std::env::var("VULKAN_SDK")
            .expect("Vulkan headers are needed for the independent FSR ABI check");
        cc::Build::new()
            .cpp(true)
            .cargo_metadata(false)
            .include("vendor/fsr-api/api/include")
            .include("vendor/fsr-api/upscalers/include")
            .include(format!("{sdk}/include"))
            .file("../../tools/ci/check_fsr_abi.cpp")
            .compile("worldcraft_fsr_abi");
    }
}

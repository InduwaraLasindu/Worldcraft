//! Offline distribution checks, before initializing a window or graphics device.
pub fn run(report: &std::path::Path) -> i32 {
    let result = (|| -> Result<(), String> {
        let text = include_str!("../../../examples/learned-highlands.worldcraft.json");
        let world: worldcraft_core::WorldFile =
            serde_json::from_str(text).map_err(|e| e.to_string())?;
        let generator = worldcraft_core::Generator::for_world(&world)?;
        if !generator.height(0.0, 0.0).is_finite() {
            return Err("Embedded example is invalid".into());
        }
        let runtime =
            crate::models::runtime_path().ok_or("Packaged native ONNX Runtime is missing")?;
        worldcraft_ml::check_runtime(&runtime)?;
        Ok(())
    })();
    let value = match &result {
        Ok(()) => {
            serde_json::json!({"status":"passed","version":env!("CARGO_PKG_VERSION"),"offline_example":true,"native_runtime":true,"renderer_initialized":false})
        }
        Err(error) => serde_json::json!({"status":"failed","error":error}),
    };
    if std::fs::write(report, serde_json::to_vec_pretty(&value).unwrap()).is_err() {
        return 2;
    }
    if result.is_ok() {
        0
    } else {
        1
    }
}

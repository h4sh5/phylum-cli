#[cfg(feature = "bindings-python")]
pub mod python;

struct CliApi;

struct PackageAnalysis {
    name: String,
    version: String,
    ecosystem: String,
    description: String,
    score: f64,
}

impl CliApi {
    fn analyze_package(name: String, version: String, ecosystem: String) -> PackageAnalysis {
        PackageAnalysis {
            name, version, ecosystem,
            description: "This package is very bad".to_string(),
            score: 0.1,
        }
    }
}

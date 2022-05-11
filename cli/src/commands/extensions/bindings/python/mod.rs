use super::{CliApi, PackageAnalysis};

use pyo3::prelude::*;
use pyo3::types::*;

#[pyclass]
struct PyCliApi(CliApi);

#[pymethods]
impl PyCliApi {
    #[new]
    fn new() -> PyCliApi {
        PyCliApi(CliApi)
    }

    fn analyze_package(
        &self,
        name: String,
        version: String,
        ecosystem: String,
    ) -> PyPackageAnalysis {
        CliApi::analyze_package(name, version, ecosystem).into()
    }
}

#[pyclass]
struct PyPackageAnalysis(PackageAnalysis);

impl From<PackageAnalysis> for PyPackageAnalysis {
    fn from(val: PackageAnalysis) -> Self {
        Self(val)
    }
}

#[pymethods]
impl PyPackageAnalysis {
    #[getter]
    fn description(&self) -> String {
        self.0.description.clone()
    }

    #[getter]
    fn score(&self) -> f64 {
        self.0.score
    }
}

fn run_extension(module_name: &str, code: &str) -> PyResult<()> {
    let gil = Python::acquire_gil();
    let py = gil.python();

    let m = PyModule::new(py, "phylum_extensions")?;
    m.add_class::<PyCliApi>()?;
    m.add_class::<PyPackageAnalysis>()?;

    py.import("sys")?
        .getattr("modules")?
        .set_item(m.name()?, m)?;

    PyModule::from_code(py, code, module_name, format!("{module_name}.py").as_str())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn runs_script() {
        run_extension(
            "extension",
            indoc! {r#"
            from phylum_extensions import PyCliApi

            api = PyCliApi()
            result = api.analyze_package("react", "1.2.3", "npm")
            print(f'Package analysis: {result.description}')
            print(f'Package score: {result.score}')
            "#},
        )
        .unwrap();
    }
}

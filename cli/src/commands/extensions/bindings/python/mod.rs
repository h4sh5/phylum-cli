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

// Sample single-module runner. If we want to give the users the chance to
// define other modules, we can just `glob` the extension directory for all
// Python files and import them manually. We are likely to not match 1:1 the
// Python import system but we should really only care to support fundamental
// use cases (e.g. `import this from that`) as opposed to module importing
// hacks.
fn run_extension(module_name: &str, code: &str) -> PyResult<()> {
    let gil = Python::acquire_gil();
    let py = gil.python();

    let m = PyModule::new(py, "phylum_extensions")?;
    m.add_class::<PyCliApi>()?;
    m.add_class::<PyPackageAnalysis>()?;

    let sys_modules = py.import("sys")?.getattr("modules")?;

    let main = py.import("__main__")?.dict();
    let builtins = main
        .get_item("__builtins__")
        .unwrap()
        .downcast::<PyModule>()?
        .dict();

    sys_modules.set_item(m.name()?, m)?;

    for (k, v) in builtins.iter() {
        println!("{k}");
    }

    if main.contains("sys")? {
        main.del_item("sys")?;
    }

    // `open` seems to be the only builtin function with side effects besides
    // `print`. we may want to blacklist other objects, but leaving `open` out
    // looks already like a successful sandbox.
    for builtin in ["open"] {
        if builtins.contains(builtin)? {
            builtins.del_item(builtin)?;
        }
    }
    for module in ["sys", "os"] {
        if sys_modules.contains(module)? {
            sys_modules.del_item(module)?;
        }
    }
    // builtins
    //     .iter()
    //     .for_each(|(k, v)| {
    //         println!("{:?}", k);
    //     });

    // 
    PyModule::from_code(py, code, module_name, format!("{module_name}.py").as_str())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::exceptions::*;

    use indoc::indoc;

    #[test]
    fn pybind_runs_script() {
        run_extension(
            "extension",
            indoc! {r#"
            import sys
            from phylum_extensions import PyCliApi

            # for m in sys.modules:
            #     print(m)

            api = PyCliApi()
            result = api.analyze_package("react", "1.2.3", "npm")
            print(f'Package analysis: {result.description}')
            print(f'Package score: {result.score}')
            "#},
        )
        .unwrap();
    }

    #[test]
    fn pybind_is_sandboxed() {
        let outcome = run_extension("extension",
            indoc! {r#"
            open('/tmp/test.txt', 'w')
            "#}
        ).unwrap_err();

        Python::with_gil(|py| {
            assert!(outcome.get_type(py).is(PyType::new::<PyNameError>(py)));
        });

        let outcome = run_extension("test",
            indoc! {r#"
            import sys

            print(sys)
            "#}
        ).unwrap_err();

        Python::with_gil(|py| {
            assert!(outcome.get_type(py).is(PyType::new::<PyKeyError>(py)));
        });
    }
}

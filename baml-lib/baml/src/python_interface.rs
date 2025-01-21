use pyo3::{create_exception, PyErr};

use crate::BamlContext;

create_exception!(baml_lib, BamlLibError, pyo3::exceptions::PyException);

impl BamlLibError {
    fn from_anyhow(err: anyhow::Error) -> PyErr {
        let backtrace = err.backtrace();
        PyErr::new::<BamlLibError, _>(format!("{}: {:?}", err, backtrace))
    }
}

#[pyo3::prelude::pyclass]
pub struct PyBamlContext {
    context: BamlContext,
}

#[pyo3::prelude::pymethods]
impl PyBamlContext {
    #[new]
    #[pyo3(signature= (schema_string, target_name=None))]
    fn new(schema_string: String, target_name: Option<String>) -> pyo3::prelude::PyResult<Self> {
        let context = BamlContext::try_from_schema(&schema_string, target_name)
            .map_err(BamlLibError::from_anyhow)?;
        Ok(PyBamlContext { context })
    }

    #[pyo3(signature = (prefix=None, always_hoist_enums=None))]
    pub fn render_prompt(
        &self,
        prefix: Option<String>,
        always_hoist_enums: Option<bool>
    ) -> pyo3::prelude::PyResult<String> {
        self.context
            .render_prompt(prefix, always_hoist_enums)
            .map_err(BamlLibError::from_anyhow)
    }

    #[pyo3(signature = (result, allow_partials=None))]
    pub fn validate_result(
        &self,
        result: String,
        allow_partials: Option<bool>
    ) -> pyo3::prelude::PyResult<String> {
        self.context
            .validate_result(&result, allow_partials.unwrap_or(false))
            .map_err(BamlLibError::from_anyhow)
    }
}


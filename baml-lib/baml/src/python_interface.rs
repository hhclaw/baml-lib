use pyo3::{create_exception, PyErr};

use crate::BamlContext;

create_exception!(baml_lib, BamlLibError, pyo3::exceptions::PyException);

impl BamlLibError {
    fn from_anyhow(err: anyhow::Error) -> PyErr {
        let backtrace = err.backtrace();
        PyErr::new::<BamlLibError, _>(format!("{}: {:?}", err, backtrace))
    }
}


#[pyo3::pyfunction]
#[pyo3(signature = (schema_string, target_name=None, prefix=None, always_hoist_enums=None))]
pub fn render_prompt(
    schema_string: String,
    target_name: Option<String>,
    prefix: Option<String>,
    always_hoist_enums: Option<bool>
) -> pyo3::prelude::PyResult<String> {
    let baml_context = BamlContext::try_from_schema(&schema_string, target_name)
        .map_err(BamlLibError::from_anyhow)?;
    baml_context
        .render_prompt(prefix, always_hoist_enums)
        .map_err(BamlLibError::from_anyhow)
}

#[pyo3::pyfunction]
#[pyo3(signature = (schema_string, result, target_name=None, allow_partials=None))]
pub fn validate_result(
    schema_string: String,
    result: String,
    target_name: Option<String>,
    allow_partials: Option<bool>
) -> pyo3::prelude::PyResult<String> {
    let baml_context = BamlContext::try_from_schema(&schema_string, target_name)
        .map_err(BamlLibError::from_anyhow)?;
    baml_context
        .validate_result(&result, allow_partials.unwrap_or(false))
        .map_err(BamlLibError::from_anyhow)
}

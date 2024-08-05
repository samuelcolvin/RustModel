use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::validators::{CombinedValidator, Validator};

mod errors;
mod field;
mod model_data;
mod model_validator;
mod validators;

#[derive(Debug)]
#[pyclass(module = "rustmodel")]
pub struct SchemaValidator {
    validator: CombinedValidator,
}

#[pymethods]
impl SchemaValidator {
    #[new]
    fn new(schema: &Bound<'_, PyDict>) -> PyResult<Self> {
        CombinedValidator::new(schema).map(|validator| Self { validator })
    }

    fn validate_python<'py>(&self, py: Python, data: &Bound<'py, PyAny>) -> PyResult<PyObject> {
        match self.validator.validate_python(py, data) {
            Ok(f) => Ok(f.into_py(py)),
            Err(e) => Err(e.to_py_err(py)),
        }
    }

    fn validate_json(&self, py: Python, json_data: &[u8]) -> PyResult<PyObject> {
        let mut jiter = jiter::Jiter::new(json_data);
        match self.validator.validate_json(py, &mut jiter) {
            Ok(f) => Ok(f.into_py(py)),
            Err(e) => Err(e.to_py_err(py)),
        }
    }

    fn __repr__(&self) -> String {
        format!("SchemaValidator(validator={:#?})", self.validator)
    }
}

#[pymodule]
fn rustmodel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SchemaValidator>()?;
    Ok(())
}

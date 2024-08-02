use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;
use std::fmt::Debug;

use crate::errors::{ErrorType, ValResult};
use crate::field::FieldValue;

pub trait Validator: Debug {
    fn validate_python<'py>(&self, py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue>;
}

#[derive(Debug, Clone)]
pub enum CombinedValidator {
    String(StringValidator),
    Int(IntValidator),
}

impl CombinedValidator {
    pub fn new(validator: &str) -> PyResult<Self> {
        match validator {
            "string" => Ok(Self::String(StringValidator)),
            "int" => Ok(Self::Int(IntValidator)),
            _ => Err(PyValueError::new_err(format!(
                "Unknown validator: {}",
                validator
            ))),
        }
    }
}

impl Validator for CombinedValidator {
    fn validate_python<'py>(&self, py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue> {
        match self {
            CombinedValidator::String(v) => v.validate_python(py, data),
            CombinedValidator::Int(v) => v.validate_python(py, data),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StringValidator;

impl Validator for StringValidator {
    fn validate_python<'py>(&self, py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue> {
        let py_str: &Bound<PyString> = data.downcast().map_err(|_| ErrorType::StringType)?;
        Ok(FieldValue::new_py(py_str.into_py(py)))
        // let s = py_str.to_str().map_err(|_| ErrorType::StringType)?;
        // Ok(FieldValue::new_raw(s))
    }
}

#[derive(Debug, Clone)]
pub struct IntValidator;

impl Validator for IntValidator {
    fn validate_python<'py>(&self, _py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue> {
        let int: i64 = data.extract().map_err(|_| ErrorType::IntType)?;
        Ok(FieldValue::new_raw(int))
    }
}

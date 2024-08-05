use jiter::{Jiter, NumberInt};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};
use std::fmt::Debug;

use crate::errors::{ErrorType, ValResult};
use crate::field::{get_as_req, FieldValue};
use crate::model_validator::ModelValidator;

pub trait Validator: Debug {
    fn validate_python<'py>(&self, py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue>;

    fn validate_json(&self, py: Python, jiter: &mut Jiter) -> ValResult<FieldValue>;
}

#[derive(Debug)]
pub enum CombinedValidator {
    String(StringValidator),
    Int(IntValidator),
    Model(ModelValidator),
}

impl CombinedValidator {
    pub fn new(schema: &Bound<'_, PyDict>) -> PyResult<Self> {
        let schema_type: String = get_as_req(schema, "type")?;
        match schema_type.as_ref() {
            "string" => Ok(Self::String(StringValidator)),
            "int" => Ok(Self::Int(IntValidator)),
            "model" => Ok(Self::Model(ModelValidator::new(schema)?)),
            _ => Err(PyValueError::new_err(format!(
                "Unknown validator: {schema_type}",
            ))),
        }
    }
}

impl Validator for CombinedValidator {
    fn validate_python<'py>(&self, py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue> {
        match self {
            CombinedValidator::String(v) => v.validate_python(py, data),
            CombinedValidator::Int(v) => v.validate_python(py, data),
            CombinedValidator::Model(v) => v.validate_python(py, data),
        }
    }
    fn validate_json(&self, py: Python, jiter: &mut Jiter) -> ValResult<FieldValue> {
        match self {
            CombinedValidator::String(v) => v.validate_json(py, jiter),
            CombinedValidator::Int(v) => v.validate_json(py, jiter),
            CombinedValidator::Model(v) => v.validate_json(py, jiter),
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

    fn validate_json(&self, _: Python, jiter: &mut Jiter) -> ValResult<FieldValue> {
        let s = jiter.next_str()?;
        Ok(FieldValue::new_raw(s))
    }
}

#[derive(Debug, Clone)]
pub struct IntValidator;

impl Validator for IntValidator {
    fn validate_python<'py>(&self, _: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue> {
        let int: i64 = data.extract().map_err(|_| ErrorType::IntType)?;
        Ok(FieldValue::new_raw(int))
    }

    fn validate_json(&self, _: Python, jiter: &mut Jiter) -> ValResult<FieldValue> {
        match jiter.next_int()? {
            NumberInt::Int(i) => Ok(FieldValue::new_raw(i)),
            NumberInt::BigInt(_) => Err(ErrorType::IntTooBig.into()),
        }
    }
}

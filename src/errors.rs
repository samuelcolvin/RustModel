use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::DowncastError;

#[pyclass(extends=PyValueError)]
#[derive(Debug)]
pub struct ValidationError {
    errors: Vec<LineError>,
}

impl ValidationError {
    pub fn new(errors: Vec<LineError>) -> Self {
        Self { errors }
    }

    pub fn new_err(py: Python, errors: Vec<LineError>) -> PyResult<PyErr> {
        let slf = Self::new(errors);
        let py_val_error = Py::new(py, slf)?;
        Ok(PyErr::from_value_bound(
            py_val_error.into_bound(py).into_any(),
        ))
    }
}

#[pymethods]
impl ValidationError {
    fn error_count(&self) -> usize {
        self.errors.len()
    }

    fn errors<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new_bound(py, self.errors.iter().map(|e| e.to_object(py)))
    }

    fn __str__(&self) -> String {
        format!("{:#?}", self.errors)
    }
}

#[derive(Debug)]
pub struct LineError {
    error_type: ErrorType,
    // reversed so that adding an "outer" location item is pushing, it's reversed before showing to the user
    rev_loc: Vec<LocItem>,
}

impl LineError {
    pub fn new_loc(error_type: ErrorType, loc: impl Into<LocItem>) -> Self {
        Self {
            error_type,
            rev_loc: vec![loc.into()],
        }
    }

    pub fn new(error_type: ErrorType) -> Self {
        Self {
            error_type,
            rev_loc: vec![],
        }
    }
}

impl ToPyObject for LineError {
    fn to_object(&self, py: Python) -> PyObject {
        let loc = self.rev_loc.iter().rev().map(|li| match li {
            LocItem::S(s) => s.to_object(py),
            LocItem::I(i) => i.to_object(py),
        });
        let error_dict = PyDict::new_bound(py);
        error_dict
            .set_item("error_type", self.error_type.to_str())
            .unwrap();
        error_dict
            .set_item("location", PyList::new_bound(py, loc))
            .unwrap();
        error_dict.into()
    }
}

#[derive(Debug)]
pub enum ErrorType {
    MissingField,
    StringType,
    StringUnicode,
    IntType,
    DictType,
}

impl ErrorType {
    fn to_str(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Clone)]
pub enum LocItem {
    S(String),
    I(i64),
}

impl From<String> for LocItem {
    fn from(s: String) -> Self {
        LocItem::S(s)
    }
}

impl From<&'_ str> for LocItem {
    fn from(s: &'_ str) -> Self {
        LocItem::S(s.to_owned())
    }
}

impl From<i64> for LocItem {
    fn from(i: i64) -> Self {
        LocItem::I(i)
    }
}

pub enum ValError {
    LineErrors(Vec<LineError>),
    InternalError(PyErr),
}

impl ValError {
    pub fn line_errors_with_loc(self, loc_into: impl Into<LocItem>) -> PyResult<Vec<LineError>> {
        match self {
            ValError::LineErrors(mut errors) => {
                let loc = loc_into.into();
                for error in errors.iter_mut() {
                    error.rev_loc.push(loc.clone());
                }
                Ok(errors)
            }
            ValError::InternalError(e) => Err(e),
        }
    }

    pub fn to_py_err(self, py: Python) -> PyErr {
        match self {
            ValError::LineErrors(errors) => {
                ValidationError::new_err(py, errors).unwrap_or_else(|e| e)
            }
            ValError::InternalError(e) => e,
        }
    }
}

pub type ValResult<T> = Result<T, ValError>;

impl From<PyErr> for ValError {
    fn from(py_err: PyErr) -> Self {
        Self::InternalError(py_err)
    }
}

impl From<DowncastError<'_, '_>> for ValError {
    fn from(py_downcast: DowncastError) -> Self {
        Self::InternalError(PyTypeError::new_err(py_downcast.to_string()))
    }
}

impl From<Vec<LineError>> for ValError {
    fn from(line_errors: Vec<LineError>) -> Self {
        Self::LineErrors(line_errors)
    }
}

impl From<LineError> for ValError {
    fn from(line_error: LineError) -> Self {
        Self::LineErrors(vec![line_error])
    }
}

impl From<ErrorType> for ValError {
    fn from(error_type: ErrorType) -> Self {
        Self::LineErrors(vec![LineError::new(error_type)])
    }
}

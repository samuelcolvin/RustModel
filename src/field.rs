use std::fmt::Debug;
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};
use serde::Serialize;
use smallvec::SmallVec;

use crate::validators::CombinedValidator;

#[derive(Debug)]
pub struct FieldInfo {
    pub name: String,
    pub name_py: Py<PyString>,
    pub required: bool,
    pub default: PyObject,
    pub validator: CombinedValidator,
}

impl FieldInfo {
    pub fn new(
        py: Python,
        name: &str,
        required: bool,
        default: PyObject,
        validator: CombinedValidator,
    ) -> Self {
        let name_py = PyString::new_bound(py, name).into_py(py);
        Self {
            name: name.to_owned(),
            name_py,
            required,
            default,
            validator,
        }
    }
}

#[derive(Debug)]
pub enum FieldValue {
    Py(PyObject),
    Model(PyObject),
    Raw(RawData),
    Both(PyObject, RawData),
}

impl FieldValue {
    pub fn new_py(py_object: PyObject) -> Self {
        FieldValue::Py(py_object)
    }

    pub fn new_raw(into_raw: impl Into<RawData>) -> Self {
        FieldValue::Raw(into_raw.into())
    }

    pub fn raw_value(&self) -> &RawData {
        match self {
            Self::Py(_) => todo!("convert PyObject to RawData"),
            Self::Model(_) => todo!("convert Model PyObject to RawData"),
            Self::Raw(raw) => raw,
            Self::Both(_, raw) => raw,
        }
    }
}

impl ToPyObject for FieldValue {
    fn to_object(&self, py: Python) -> PyObject {
        match self {
            Self::Py(py_obj) => py_obj.clone_ref(py),
            Self::Model(py_obj) => py_obj.clone_ref(py),
            Self::Raw(raw) => raw.to_object(py),
            Self::Both(py_obj, _) => py_obj.clone_ref(py),
        }
    }
}

impl IntoPy<PyObject> for FieldValue {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Py(py_obj) => py_obj,
            Self::Model(py_obj) => py_obj,
            Self::Raw(raw) => raw.to_object(py),
            Self::Both(py_obj, _) => py_obj,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RawData {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(RawList),
    Dict(RawDict),
}

type RawList = Arc<SmallVec<[RawData; 8]>>;
type RawDict = Arc<SmallVec<[(String, RawData); 8]>>;

impl Serialize for RawData {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            RawData::None => serializer.serialize_none(),
            RawData::Bool(b) => serializer.serialize_bool(*b),
            RawData::Int(i) => serializer.serialize_i64(*i),
            RawData::Float(f) => serializer.serialize_f64(*f),
            RawData::Str(s) => serializer.serialize_str(s),
            RawData::List(l) => l.serialize(serializer),
            RawData::Dict(d) => d.serialize(serializer),
        }
    }
}

impl ToPyObject for RawData {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        match self {
            Self::None => py.None().to_object(py),
            Self::Bool(b) => b.to_object(py),
            Self::Int(i) => i.to_object(py),
            Self::Float(f) => f.to_object(py),
            Self::Str(s) => s.to_object(py),
            Self::List(v) => PyList::new_bound(py, v.iter().map(|v| v.to_object(py))).to_object(py),
            Self::Dict(o) => {
                let dict = PyDict::new_bound(py);
                for (k, v) in o.iter() {
                    dict.set_item(k, v.to_object(py)).unwrap();
                }
                dict.to_object(py)
            }
        }
    }
}

impl From<bool> for RawData {
    fn from(v: bool) -> Self {
        RawData::Bool(v)
    }
}

impl From<i64> for RawData {
    fn from(v: i64) -> Self {
        RawData::Int(v)
    }
}

impl From<f64> for RawData {
    fn from(v: f64) -> Self {
        RawData::Float(v)
    }
}

impl From<String> for RawData {
    fn from(v: String) -> Self {
        RawData::Str(v)
    }
}

impl From<&str> for RawData {
    fn from(v: &str) -> Self {
        RawData::Str(v.to_owned())
    }
}

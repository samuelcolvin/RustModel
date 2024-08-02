use std::sync::Arc;

use pyo3::exceptions::{PyAttributeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString};

use crate::field::{FieldInfo, FieldValue};
use ahash::AHashMap;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::Serialize;

#[derive(Debug)]
#[pyclass(module="fastmodel")]
pub struct ModelData {
    field_info: Arc<Vec<FieldInfo>>,
    field_data: Vec<Option<FieldValue>>,
    key_lookup: Arc<AHashMap<String, usize>>,
}

#[pymethods]
impl ModelData {
    fn get_attr(&self, py: Python, key: String) -> PyResult<PyObject> {
        if let Some(index) = self.key_lookup.get(&key) {
            match &self.field_data[*index] {
                Some(f) => Ok(f.python_value(py)),
                None => Ok(py.None()),
            }
        } else {
            Err(PyAttributeError::new_err(key))
        }
    }

    fn model_dump(&mut self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (field_info, field_value) in self.items_update(py) {
            dict.set_item(
                &field_info.name_py.clone_ref(py),
                field_value.python_value(py),
            )?;
        }
        Ok(dict.into())
    }

    fn model_dump_json(&self, py: Python) -> PyResult<String> {
        let model_data_serializer = ModelDataSerializer {
            py,
            field_info: &self.field_info,
            field_data: &self.field_data,
        };
        serde_json::to_string(&model_data_serializer)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

impl ModelData {
    pub fn new(
        field_info: &Arc<Vec<FieldInfo>>,
        field_data: Vec<Option<FieldValue>>,
        key_lookup: &Arc<AHashMap<String, usize>>,
    ) -> Self {
        Self {
            field_info: field_info.clone(),
            field_data,
            key_lookup: key_lookup.clone(),
        }
    }

    fn items_update<'py>(
        &'py mut self,
        py: Python<'py>,
    ) -> impl Iterator<Item = (&FieldInfo, &FieldValue)> + 'py {
        self.field_info
            .iter()
            .zip(self.field_data.iter_mut())
            .map(move |(f, v)| {
                let value = match v {
                    Some(f) => f,
                    None => {
                        *v = Some(FieldValue::new_py(f.default.clone_ref(py)));
                        v.as_ref().unwrap()
                    }
                };
                (f, value)
            })
    }
}

struct ModelDataSerializer<'py> {
    py: Python<'py>,
    field_info: &'py Arc<Vec<FieldInfo>>,
    field_data: &'py Vec<Option<FieldValue>>,
}

impl Serialize for ModelDataSerializer<'_> {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.field_data.len()))?;

        let items = self.field_info.iter().zip(self.field_data.iter());

        for (field_info, opt_field_value) in items {
            if let Some(field_value) = opt_field_value {
                map.serialize_entry(&field_info.name, field_value.raw_value())?;
            } else {
                let py_data = PyData(field_info.default.clone_ref(self.py).into_bound(self.py));
                map.serialize_entry(&field_info.name, &py_data)?;
            }
        }
        map.end()
    }
}

struct PyData<'py>(Bound<'py, PyAny>);

impl Serialize for PyData<'_> {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let py_value = &self.0;
        if py_value.is_none() {
            serializer.serialize_none()
        } else if let Ok(value) = py_value.downcast::<PyBool>() {
            serializer.serialize_bool(value.is_true())
        } else if let Ok(value) = py_value.downcast::<PyString>() {
            let s = value.to_str().map_err(serde::ser::Error::custom)?;
            serializer.serialize_str(s)
        } else if let Ok(value) = py_value.downcast::<PyInt>() {
            serializer.serialize_i64(value.extract::<i64>().map_err(serde::ser::Error::custom)?)
        } else if let Ok(value) = py_value.downcast::<PyFloat>() {
            serializer.serialize_f64(value.extract::<f64>().map_err(serde::ser::Error::custom)?)
        } else if let Ok(value) = py_value.downcast::<PyList>() {
            let mut list_ser = serializer.serialize_seq(Some(value.len()))?;
            for item in value.iter() {
                list_ser.serialize_element(&PyData(item))?;
            }
            list_ser.end()
        } else if let Ok(value) = py_value.downcast::<PyDict>() {
            let mut map_ser = serializer.serialize_map(Some(value.len()))?;
            for (key, value) in value.iter() {
                let key = key
                    .downcast::<PyString>()
                    .map_err(serde::ser::Error::custom)?
                    .to_str()
                    .map_err(serde::ser::Error::custom)?;
                map_ser.serialize_entry(&key, &PyData(value))?;
            }
            map_ser.end()
        } else {
            Err(serde::ser::Error::custom("unsupported type"))
        }
    }
}

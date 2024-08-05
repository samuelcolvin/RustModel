use std::fmt::Debug;
use std::ptr::null_mut;
use std::sync::Arc;

use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple, PyType};

use ahash::AHashMap;

use crate::errors::{ErrorType, LineError, ValResult};
use crate::field::{get_as_req, parse_fields, FieldInfo, FieldValue};
use crate::model_data::ModelData;
use crate::validators::Validator;

#[derive(Debug)]
pub struct ModelValidator {
    field_info: Arc<Vec<FieldInfo>>,
    key_lookup: Arc<AHashMap<String, usize>>,
    cls: Py<PyType>,
}

impl ModelValidator {
    pub fn new(schema: &Bound<'_, PyDict>) -> PyResult<Self> {
        let fields = get_as_req(schema, "fields")?;
        let field_info = parse_fields(schema.py(), fields)?;
        let key_lookup: AHashMap<String, usize> = field_info
            .iter()
            .enumerate()
            .map(|(i, f)| (f.name.clone(), i))
            .collect();

        let class: Bound<PyType> = get_as_req(schema, "cls")?;

        Ok(Self {
            field_info: Arc::new(field_info),
            key_lookup: Arc::new(key_lookup),
            cls: class.into(),
        })
    }
}

impl Validator for ModelValidator {
    fn validate_python<'py>(&self, py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue> {
        let dict = data.downcast::<PyDict>().map_err(|_| ErrorType::DictType)?;
        let mut errors: Vec<LineError> = Vec::new();

        let field_count = self.field_info.len();
        // can't clone `FieldValue`
        let mut data: Vec<Option<FieldValue>> = (0..field_count).map(|_| None).collect();
        let mut fields_found = 0;

        for (key, value) in dict.iter() {
            if let Ok(key_py_str) = key.downcast::<PyString>() {
                let key_str = key_py_str.to_str()?;
                if let Some(index) = self.key_lookup.get(key_str) {
                    let field_info = &self.field_info[*index];
                    match field_info.validator.validate_python(py, &value) {
                        Ok(field_value) => {
                            data[*index] = Some(field_value);
                            fields_found += 1;
                        }
                        Err(e) => errors.extend(e.line_errors_with_loc(key_str)?),
                    }
                }
            }
        }

        if fields_found != field_count {
            for (info, value) in self.field_info.iter().zip(data.iter()) {
                if value.is_none() && info.required {
                    errors.push(LineError::new_loc(
                        ErrorType::MissingField,
                        info.name.as_str(),
                    ));
                }
            }
        }

        let instance = create_class(self.cls.bind(py))?;

        if errors.is_empty() {
            let model_data = ModelData::new(&self.field_info, data, &self.key_lookup);
            force_setattr(
                py,
                &instance,
                intern!(py, "__pydantic_model_data__"),
                Py::new(py, model_data)?,
            )?;
            Ok(FieldValue::Model(instance.into_py(py)))
        } else {
            Err(errors.into())
        }
    }
}

/// The rest here is taken directly from pydantic-core
pub(super) fn create_class<'py>(class: &Bound<'py, PyType>) -> PyResult<Bound<'py, PyAny>> {
    let py = class.py();
    let args = PyTuple::empty_bound(py);
    let raw_type = class.as_type_ptr();
    unsafe {
        // Safety: raw_type is known to be a non-null type object pointer
        match (*raw_type).tp_new {
            // Safety: the result of new_func is guaranteed to be either an owned pointer or null on error returns.
            Some(new_func) => Bound::from_owned_ptr_or_err(
                py,
                // Safety: the non-null pointers are known to be valid, and it's allowed to call tp_new with a
                // null kwargs dict.
                new_func(raw_type, args.as_ptr(), null_mut()),
            ),
            None => Err(PyTypeError::new_err("base type without tp_new")),
        }
    }
}

pub(super) fn force_setattr<N, V>(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    attr_name: N,
    value: V,
) -> PyResult<()>
where
    N: ToPyObject,
    V: ToPyObject,
{
    let attr_name = attr_name.to_object(py);
    let value = value.to_object(py);
    unsafe {
        py_error_on_minusone(
            py,
            pyo3::ffi::PyObject_GenericSetAttr(obj.as_ptr(), attr_name.as_ptr(), value.as_ptr()),
        )
    }
}

pub fn py_error_on_minusone(py: Python<'_>, result: std::os::raw::c_int) -> PyResult<()> {
    if result != -1 {
        Ok(())
    } else {
        Err(PyErr::fetch(py))
    }
}

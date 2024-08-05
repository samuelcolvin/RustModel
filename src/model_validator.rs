use std::fmt::Debug;
use std::ptr::null_mut;
use std::sync::Arc;

use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple, PyType};

use ahash::AHashMap;
use jiter::Jiter;

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
        ModelValidate::new(self).validate_python(py, data)
    }

    fn validate_json(&self, py: Python, jiter: &mut Jiter) -> ValResult<FieldValue> {
        ModelValidate::new(self).validate_json(py, jiter)
    }
}

struct ModelValidate<'a> {
    validator: &'a ModelValidator,
    errors: Vec<LineError>,
    data: Vec<Option<FieldValue>>,
    field_count: usize,
    fields_found: usize,
}

impl<'a> ModelValidate<'a> {
    fn new(validator: &'a ModelValidator) -> Self {
        let field_count = validator.field_info.len();
        Self {
            validator,
            errors: Vec::new(),
            // can't clone `FieldValue`
            data: (0..field_count).map(|_| None).collect(),
            field_count,
            fields_found: 0,
        }
    }

    fn validate_python<'py>(mut self, py: Python, data: &Bound<'py, PyAny>) -> ValResult<FieldValue> {
        let dict = data.downcast::<PyDict>().map_err(|_| ErrorType::DictType)?;

        for (key, value) in dict.iter() {
            if let Ok(key_py_str) = key.downcast::<PyString>() {
                let key_str = key_py_str.to_str()?;
                if let Some((index, field_info)) = self.find_validator(key_str) {
                    match field_info.validator.validate_python(py, &value) {
                        Ok(field_value) => self.set_value(index, field_value),
                        Err(e) => self.errors.extend(e.line_errors_with_loc(key_str)?),
                    }
                }
            }
        }

        self.finish(py)
    }

    fn validate_json(mut self, py: Python, jiter: &mut Jiter) -> ValResult<FieldValue> {
        if let Some(first_key) = jiter.next_object()? {
            self.validate_json_field(py, first_key.to_string(), jiter)?;

            while let Some(key) = jiter.next_key()? {
                self.validate_json_field(py, key.to_string(), jiter)?;
            }
        }

        self.finish(py)
    }

    fn validate_json_field(&mut self, py: Python, k: String, jiter: &mut Jiter) -> ValResult<()> {
        if let Some((index, field_info)) = self.find_validator(&k) {
            match field_info.validator.validate_json(py, jiter) {
                Ok(field_value) => self.set_value(index, field_value),
                Err(e) => self.errors.extend(e.line_errors_with_loc(k.as_str())?),
            };
        } else {
            jiter.next_skip()?;
        }
        Ok(())
    }

    fn find_validator(&self, key: &str) -> Option<(usize, &FieldInfo)> {
        self.validator.key_lookup.get(key).map(|index| (*index, &self.validator.field_info[*index]))
    }

    fn set_value(&mut self, index: usize, value: FieldValue) {
        self.data[index] = Some(value);
        self.fields_found += 1;
    }

    fn finish(mut self, py: Python) -> ValResult<FieldValue> {
        if self.fields_found != self.field_count {
            for (info, value) in self.validator.field_info.iter().zip(self.data.iter()) {
                if value.is_none() && info.required {
                    self.errors.push(LineError::new_loc(
                        ErrorType::MissingField,
                        info.name.as_str(),
                    ));
                }
            }
        }

        let instance = create_class(self.validator.cls.bind(py))?;

        if self.errors.is_empty() {
            let model_data = ModelData::new(&self.validator.field_info, self.data, &self.validator.key_lookup);
            force_setattr(
                py,
                &instance,
                intern!(py, "__pydantic_model_data__"),
                Py::new(py, model_data)?,
            )?;
            Ok(FieldValue::Model(instance.into_py(py)))
        } else {
            Err(self.errors.into())
        }
    }
}

/// The rest here is taken directly from pydantic-core
fn create_class<'py>(class: &Bound<'py, PyType>) -> PyResult<Bound<'py, PyAny>> {
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

fn force_setattr(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    attr_name: impl ToPyObject,
    value: impl ToPyObject,
) -> PyResult<()> {
    let attr_name = attr_name.to_object(py);
    let value = value.to_object(py);
    unsafe {
        py_error_on_minusone(
            py,
            pyo3::ffi::PyObject_GenericSetAttr(obj.as_ptr(), attr_name.as_ptr(), value.as_ptr()),
        )
    }
}

fn py_error_on_minusone(py: Python<'_>, result: std::os::raw::c_int) -> PyResult<()> {
    if result != -1 {
        Ok(())
    } else {
        Err(PyErr::fetch(py))
    }
}

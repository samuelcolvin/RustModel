use pyo3::prelude::*;

mod errors;
mod field;
mod model_data;
mod model_validator;
mod validators;

#[pymodule]
fn fastmodel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<model_validator::ModelValidator>()?;
    Ok(())
}

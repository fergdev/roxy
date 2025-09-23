use pyo3::{
    Bound, pyclass, pymethods,
    types::{PyDict, PyTuple},
};

#[pyclass(subclass)]
pub struct Extension {}

#[pymethods]
impl Extension {
    #[new]
    #[pyo3(signature = (*args, **kwargs))]
    #[allow(unused_variables)]
    fn new(args: &Bound<PyTuple>, kwargs: Option<Bound<PyDict>>) -> Self {
        Extension {}
    }
}

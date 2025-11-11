use crate::{Filter, FilterImpl, FilterOptions, FilterOutput, TokenIDsWithLogProb, new_filter};
use pyo3::prelude::*;

#[pyclass]
struct PyFilter {
    inner: FilterImpl,
}

#[pymethods]
impl PyFilter {
    #[new]
    fn new(opts: &PyFilterOptions) -> Self {
        PyFilter {
            inner: new_filter(opts.inner.clone()),
        }
    }

    // TODO: figure out how we want to pass log probs (if we do)
    fn write_decoded(&mut self, decoded_token: &str) -> Vec<FilterOutput> {
        self.inner
            .write_decoded(decoded_token, TokenIDsWithLogProb::new())
    }

    fn flush_partials(&mut self) -> Vec<FilterOutput> {
        self.inner.flush_partials()
    }
}

#[pyclass]
struct PyFilterOptions {
    inner: FilterOptions,
}

#[pymethods]
impl PyFilterOptions {
    #[new]
    fn new() -> Self {
        PyFilterOptions {
            inner: FilterOptions::default(),
        }
    }

    fn cmd3(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = std::mem::take(&mut slf.inner).cmd3();
        slf
    }

    fn cmd4(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = std::mem::take(&mut slf.inner).cmd4();
        slf
    }

    fn remove_token<'a>(mut slf: PyRefMut<'a, Self>, token: &str) -> PyRefMut<'a, Self> {
        slf.inner = std::mem::take(&mut slf.inner).remove_token(token);
        slf
    }
}

#[pymodule]
fn cohere_melody(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFilter>()?;
    m.add_class::<PyFilterOptions>()?;
    Ok(())
}

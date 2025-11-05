use pyo3::prelude::*;
use crate::{new_filter, Filter, FilterImpl, FilterOptions, FilterOutput, TokenIDsWithLogProb};

#[pyclass]
struct PyFilter {
    inner: FilterImpl,
}

#[pymethods]
impl PyFilter {
    #[new]
    fn new(opts: PyRefMut<PyFilterOptions>) -> Self {
        PyFilter {
            inner: new_filter(opts.inner.clone()),
        }
    }

    // TODO: figure out how we want to pass log probs (if we do)
    fn write_decoded(&mut self, decoded_token: &str) -> PyResult<Vec<FilterOutput>> {
        // You may need to convert logprobs to TokenIDsWithLogProb as appropriate
        Ok(self.inner.write_decoded(decoded_token, TokenIDsWithLogProb::new()))
    }

    fn flush_partials(&mut self) -> PyResult<Vec<FilterOutput>> {
        Ok(self.inner.flush_partials())
    }
}

#[pyclass]
struct PyFilterOptions {
    inner: FilterOptions
}

#[pymethods]
impl PyFilterOptions {
    #[new]
    fn new() -> Self {
        PyFilterOptions {
            inner: FilterOptions::default()
        }
    }

    fn handle_multi_hop_cmd3(mut slf: PyRefMut<Self>) -> PyResult<PyRefMut<Self>> {
        slf.inner = slf.inner.clone().handle_multi_hop_cmd3();
        Ok(slf)
    }

    fn stream_tool_actions(mut slf: PyRefMut<Self>) -> PyResult<PyRefMut<Self>> {
        slf.inner = slf.inner.clone().stream_tool_actions();
        Ok(slf)
    }
}

#[pymodule]
fn cohere_melody(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFilter>()?;
    m.add_class::<PyFilterOptions>()?;
    Ok(())
}
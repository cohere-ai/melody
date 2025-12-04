//! Python bindings for the Melody parsing library
//!
//! This module provides Python bindings using PyO3, allowing the Melody parser
//! to be used directly from Python code.

use crate::{Filter, FilterImpl, FilterOptions, FilterOutput, TokenIDsWithLogProb, new_filter};
use pyo3::prelude::*;

/// Python wrapper for the streaming filter.
///
/// This class provides the main interface for parsing model outputs from Python.
/// Create an instance with `PyFilterOptions` and then call `write_decoded` for
/// each token as it arrives.
#[pyclass]
struct PyFilter {
    inner: FilterImpl,
}

#[pymethods]
impl PyFilter {
    /// Create a new filter with the given options.
    ///
    /// Args:
    ///     opts: PyFilterOptions instance with desired configuration
    ///
    /// Returns:
    ///     A new PyFilter instance
    #[new]
    fn new(opts: &PyFilterOptions) -> Self {
        PyFilter {
            inner: new_filter(opts.inner.clone()),
        }
    }

    /// Process a decoded token and return any completed outputs.
    ///
    /// Args:
    ///     decoded_token: The decoded text for this token
    ///
    /// Returns:
    ///     List of FilterOutput objects (may be empty if content is buffered)
    ///
    /// Note:
    ///     Log probabilities are not currently supported in the Python API
    fn write_decoded(&mut self, decoded_token: &str) -> Vec<FilterOutput> {
        self.inner
            .write_decoded(decoded_token, TokenIDsWithLogProb::new())
    }

    /// Flush any buffered partial outputs.
    ///
    /// Call this at the end of generation to output any content that was
    /// buffered waiting for special tokens.
    ///
    /// Returns:
    ///     List of remaining FilterOutput objects
    fn flush_partials(&mut self) -> Vec<FilterOutput> {
        self.inner.flush_partials()
    }
}

/// Python wrapper for filter configuration options.
///
/// This class provides a builder pattern for configuring filter behavior.
/// Use preset methods like `cmd3()` or customize with individual setters.
#[pyclass]
struct PyFilterOptions {
    inner: FilterOptions,
}

#[pymethods]
impl PyFilterOptions {
    /// Create a new options instance with default settings.
    ///
    /// Returns:
    ///     A new PyFilterOptions instance
    #[new]
    fn new() -> Self {
        PyFilterOptions {
            inner: FilterOptions::default(),
        }
    }

    /// Configure for Cohere Command 3 model format.
    ///
    /// Enables grounded answer parsing, tool action streaming, and Command 3-style citations.
    ///
    /// Returns:
    ///     Self (for method chaining)
    fn cmd3(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = std::mem::take(&mut slf.inner).cmd3();
        slf
    }

    /// Configure for Cohere Command 4 model format.
    ///
    /// Similar to Command 3 but with different special token markers.
    ///
    /// Returns:
    ///     Self (for method chaining)
    fn cmd4(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = std::mem::take(&mut slf.inner).cmd4();
        slf
    }

    /// Remove a special token from the configuration.
    ///
    /// Args:
    ///     token: The special token string to remove
    ///
    /// Returns:
    ///     Self (for method chaining)
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

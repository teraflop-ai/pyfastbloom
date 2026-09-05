use fastbloom::AtomicBloomFilter as Inner;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyclass(frozen, module = "pyfastbloom")]
struct BloomFilter {
    inner: Inner,
    seed: u128,
}

impl BloomFilter {
    fn hash(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(if let Ok(s) = item.extract::<&str>() {
            self.inner.source_hash(s.as_bytes())
        } else if let Ok(b) = item.extract::<&[u8]>() {
            self.inner.source_hash(b)
        } else {
            self.inner.source_hash(&item.extract::<i128>()?)
        })
    }

    fn compatible(&self, other: &Self) -> PyResult<()> {
        if self.seed != other.seed
            || self.inner.num_bits() != other.inner.num_bits()
            || self.inner.num_hashes() != other.inner.num_hashes()
        {
            return Err(PyValueError::new_err("filters have different seed, size or hashes"));
        }
        Ok(())
    }
}

#[pymethods]
impl BloomFilter {
    #[new]
    #[pyo3(signature = (expected_items, false_positive_rate = 0.01, *, num_bits = None, seed = None))]
    fn new(expected_items: usize, false_positive_rate: f64, num_bits: Option<usize>, seed: Option<u128>) -> PyResult<Self> {
        if !(false_positive_rate > 0.0) || num_bits == Some(0) {
            return Err(PyValueError::new_err("false_positive_rate and num_bits must be > 0"));
        }
        let seed = seed.unwrap_or_else(rand::random);
        let inner = match num_bits {
            Some(n) => Inner::with_num_bits(n).seed(&seed).expected_items(expected_items),
            None => Inner::with_false_pos(false_positive_rate).seed(&seed).expected_items(expected_items),
        };
        Ok(Self { inner, seed })
    }

    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        if data.len() < 28 || (data.len() - 20) % 8 != 0 {
            return Err(PyValueError::new_err("malformed data"));
        }
        let seed = u128::from_le_bytes(data[..16].try_into().unwrap());
        let hashes = u32::from_le_bytes(data[16..20].try_into().unwrap());
        let words = data[20..].chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
        Ok(Self { inner: Inner::from_vec(words).seed(&seed).hashes(hashes), seed })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let mut v = Vec::with_capacity(20 + self.inner.num_bits() / 8);
        v.extend_from_slice(&self.seed.to_le_bytes());
        v.extend_from_slice(&self.inner.num_hashes().to_le_bytes());
        self.inner.iter().for_each(|w| v.extend_from_slice(&w.to_le_bytes()));
        PyBytes::new(py, &v)
    }

    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (Bound<'py, PyBytes>,))> {
        Ok((py.get_type::<Self>().getattr("from_bytes")?, (self.to_bytes(py),)))
    }

    fn insert(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.insert_hash(self.hash(item)?))
    }

    fn update(&self, items: &Bound<'_, PyAny>) -> PyResult<()> {
        items.try_iter()?.try_for_each(|i| self.insert(&i?).map(drop))
    }

    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains_hash(self.hash(item)?))
    }

    fn __contains__(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.contains(item)
    }

    fn clear(&self) {
        self.inner.clear()
    }

    fn union(&self, other: &Self) -> PyResult<()> {
        self.compatible(other)?;
        Ok(self.inner.union(&other.inner))
    }

    fn intersect(&self, other: &Self) -> PyResult<()> {
        self.compatible(other)?;
        Ok(self.inner.intersect(&other.inner))
    }

    fn expected_false_positive_rate(&self, num_items: usize) -> f64 {
        self.inner.expected_false_pos(num_items)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.seed == other.seed && self.inner == other.inner
    }

    #[getter]
    fn num_bits(&self) -> usize {
        self.inner.num_bits()
    }

    #[getter]
    fn num_hashes(&self) -> u32 {
        self.inner.num_hashes()
    }

    #[getter]
    fn seed(&self) -> u128 {
        self.seed
    }
}

#[pymodule]
fn pyfastbloom(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<BloomFilter>()
}
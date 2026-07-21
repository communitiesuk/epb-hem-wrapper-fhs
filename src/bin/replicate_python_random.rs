#[cfg(test)]
use anyhow::anyhow;
#[cfg(test)]

use pyo3::prelude::*;

fn main() {

}

#[cfg(test)]
fn random_number_generation() -> anyhow::Result<Vec<f64>> {
    let rng = Python::attach(|py| -> PyResult<Py<PyAny>> {
        make_rng(py, 37)
    }).map_err( |e| anyhow!(e))?;
    let mut random_numbers = Vec::with_capacity(10);

    for _ in 0..10 {
        let random = Python::attach(|py| -> PyResult<f64> {
            random(py, rng.clone_ref(py))
        }).map_err( |e| anyhow!(e))?;
        random_numbers.push(random);
    }

    Ok(random_numbers)
}

#[cfg(test)]
fn poisson_number_generation() -> anyhow::Result<Vec<Vec<i32>>> {
   let rng_poisson = Python::attach(|py| -> PyResult<Py<PyAny>> {
           make_rng_poisson(py, 37)
       }).map_err( |e| anyhow!(e))?;
    let mut random_numbers = Vec::with_capacity(10);

    for i in 0..10 {
        let random = Python::attach(|py| -> PyResult<Vec<i32>> {
            rng_poisson.call_method1(py, "poisson", (i as f64/10., 53))?.call_method0(py, "tolist")?.extract(py)

        }).map_err( |e| anyhow!(e))?;
        random_numbers.push(random);
    }

    Ok(random_numbers)

}


#[cfg(test)]
#[pyfunction]
fn make_rng(py: Python<'_>, seed: u64) -> PyResult<Py<PyAny>> {
    let random = py.import("random")?;
    let rng = random.getattr("Random")?.call1((seed,))?;
    Ok(rng.unbind())
}

#[cfg(test)]
#[pyfunction]
fn random(py: Python<'_>, rng: Py<PyAny>) -> PyResult<f64> {
    rng.call_method0(py, "random")?.extract::<f64>(py)
}


#[cfg(test)]
#[pyfunction]
fn make_rng_poisson(py: Python<'_>, seed: u64) -> PyResult<Py<PyAny>> {
    let np_random = py.import("numpy.random")?;
    let rng_poisson = np_random.getattr("default_rng")?.call1((seed,))?;
    Ok(rng_poisson.unbind())
}

#[cfg(test)]
mod test {
    use crate::{main, poisson_number_generation, random_number_generation};

    #[test]
    fn test_random_number_generation() {
        let actual = random_number_generation().unwrap();
        let expected = vec!(
            0.6820045605879779,
            0.09160260807956389,
            0.6178163488614024,
            0.8419199045509562,
            0.8345502885760898,
            0.5150177257913494,
            0.6310379652956766,
            0.36922983406291854,
            0.5280186220192247,
            0.1078566833027319,
        );
        assert_eq!(actual, expected);

    }
    #[test]
    fn test_poisson_number_generation() {
        let actual = poisson_number_generation().unwrap();
        let expected = vec!(
            Vec::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Vec::from([0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]),
            Vec::from([0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0]),
            Vec::from([0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0]),
            Vec::from([0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 2, 0, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 0]),
            Vec::from([0, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 2, 0, 1, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 2, 0, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0]),
            Vec::from([2, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 3, 2, 0, 0, 1, 1, 2, 1, 0, 1, 0, 0, 0, 1, 0, 0, 3, 0, 0, 0, 1, 0, 1, 0, 0, 4, 2, 0, 0, 0, 1, 0, 1, 1, 1, 1, 0]),
            Vec::from([0, 0, 1, 1, 1, 0, 0, 1, 1, 0, 2, 0, 0, 0, 4, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 2, 0, 1, 0, 0, 1, 0, 2, 1, 1, 2, 1, 1, 0, 1, 3, 0, 0, 1, 0, 2, 0, 0, 1, 0, 1, 0, 0]),
            Vec::from([1, 0, 2, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 1, 2, 2, 0, 0, 1, 2, 0, 1, 0, 2, 1, 0, 1, 1, 2, 1, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 1, 0, 0, 1, 2, 1, 1, 2, 0, 0, 0, 3, 2]),
            Vec::from([1, 2, 2, 1, 1, 2, 0, 2, 3, 0, 2, 0, 0, 1, 4, 1, 0, 3, 0, 2, 1, 0, 1, 0, 1, 0, 2, 1, 1, 1, 1, 1, 1, 2, 2, 3, 0, 1, 1, 0, 0, 1, 2, 1, 0, 0, 0, 1, 1, 2, 3, 0, 0]),
        );
        assert_eq!(expected, actual)
    }
}


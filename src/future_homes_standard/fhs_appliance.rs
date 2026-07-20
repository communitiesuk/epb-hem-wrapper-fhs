use anyhow::anyhow;
use home_energy_model::{
    core::units::{DAYS_PER_YEAR, HOURS_PER_DAY, WATTS_PER_KILOWATT},
    input::ApplianceGainsEvent,
};
use pyo3::prelude::*;
use pyo3::{pyfunction, Py, PyAny, PyResult, Python};

pub(super) struct FhsAppliance {
    pub(super) standby_w: f64,
    pub(super) gains_frac: f64,
    pub(super) event_list: Vec<ApplianceGainsEvent>,
}

impl FhsAppliance {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        util_unit: f64,
        annual_use_per_unit: f64,
        op_kwh: f64,
        event_duration: f64,
        standby_w: f64,
        gains_frac: f64,
        flat_profile: &[f64],
        seed: Option<usize>,
        duration_std_dev: Option<f64>,
    ) -> anyhow::Result<Self> {
        let annual_expected_uses = util_unit * annual_use_per_unit;
        let seed = seed.unwrap_or(DEFAULT_SEED);
        let duration_std_dev = duration_std_dev.unwrap_or(DEFAULT_DURATION_STD_DEV);
        let annual_expected_demand = annual_expected_uses * op_kwh
            + standby_w
                * ((HOURS_PER_DAY * DAYS_PER_YEAR) as f64 - annual_expected_uses * event_duration)
                / WATTS_PER_KILOWATT as f64;
        let (event_list, _flat_schedule) = Self::build_sched(
            flat_profile,
            seed,
            annual_expected_uses,
            annual_expected_demand,
            op_kwh,
            standby_w,
            event_duration,
            duration_std_dev,
        )?;
        Ok(Self {
            standby_w,
            gains_frac,
            event_list,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn build_sched(
        flat_profile: &[f64],
        seed: usize,
        annual_expected_uses: f64,
        annual_expected_demand: f64,
        op_kwh: f64,
        standby_w: f64,
        event_duration: f64,
        duration_std_dev: f64,
    ) -> anyhow::Result<(Vec<ApplianceGainsEvent>, Vec<f64>)> {
        // upstream Python here constructs a seed sequence from consecutive numbers - instead, here we sum the series
        #[pyfunction]
        fn make_default_rng(py: Python<'_>, seed: Vec<u64>) -> PyResult<Py<PyAny>> {
            let np_random = py.import("numpy.random")?;
            let rng = np_random.getattr("default_rng")?.call1((seed,))?;
            Ok(rng.unbind())
        }
        let flat_profile_len = flat_profile.len();

        let appliance_rng = Python::attach(|py| -> PyResult<Py<PyAny>> {
            make_default_rng(
                py,
                (0..(flat_profile_len + annual_expected_uses.ceil() as usize))
                    .map(|x| (x + seed) as u64)
                    .collect(),
            )
        })
        .map_err(|e| anyhow!(e))?;

        let lambda: Vec<f64> = flat_profile
            .iter()
            .map(|x| x * annual_expected_uses / DAYS_PER_YEAR as f64)
            .collect();
        let events = Python::attach(|py| -> PyResult<Vec<f64>> {
            appliance_rng
                .call_method1(py, "poisson", (lambda, flat_profile_len))?
                .extract(py)
        })?;

        let num_events = events.iter().copied().map(|x| x as usize).sum::<usize>();
        let mut event_size_deviations = Python::attach(|py| -> PyResult<Vec<f64>> {
            appliance_rng
                .call_method1(py, "normal", (0, duration_std_dev, num_events))?
                .extract(py)
        })?;

        for deviation in event_size_deviations.iter_mut() {
            if *deviation < -1. {
                let normal = Python::attach(|py| -> PyResult<f64> {
                    appliance_rng
                        .call_method1(py, "normal", (0, duration_std_dev))?
                        .extract(py)
                })?;
                *deviation = normal.max(-1.);
            }
        }

        let norm_events = num_events as f64 + event_size_deviations.iter().sum::<f64>();
        // adjustment in mean event length  to account for random variation
        // adjustment is not applied to standby power,
        // the total demand of which depends on length of events
        // expect sufficient convergence after 10 iterations
        let mut f_appliance = 1.;
        let convergence_threshold = 0.0000001;
        for _i in 0..10 {
            let f_appliance_test = f_appliance;
            f_appliance = (norm_events * op_kwh)
                / (annual_expected_demand
                    - standby_w
                        * ((HOURS_PER_DAY * DAYS_PER_YEAR) as f64
                            - norm_events * event_duration / f_appliance)
                        / WATTS_PER_KILOWATT as f64);
            if (f_appliance - f_appliance_test).abs() < convergence_threshold {
                // break out of loop if Fappliance does not change by more than convergence threshold
                break;
            }
        }

        // TODO (from Python) - analytical method is simpler than the above:
        //             P_e = self.op_kWh * W_per_kW /(self.event_duration)
        //             Fappliance = ((P_e - self.standby_W)* self.event_duration * norm_events/ W_per_kW)\
        //                         / (self.annual_expected_demand - hours_per_day * days_per_year * self.standby_W / W_per_kW)

        let expected_demand_w_event = op_kwh * WATTS_PER_KILOWATT as f64 / event_duration;
        let mut eventlist: Vec<ApplianceGainsEvent> = vec![];
        let mut sched = vec![standby_w; flat_profile_len];

        let mut event_count: usize = Default::default();
        for (step, num_events_in_step) in events.into_iter().enumerate() {
            let mut start_offset = Python::attach(|py| -> PyResult<f64> {
                appliance_rng.call_method0(py, "random")?.extract(py)
            })?;
            for e in 0..(num_events_in_step.floor() as usize) {
                let demand_w_event = expected_demand_w_event;
                let duration =
                    event_duration * (1. + event_size_deviations[event_count]) / f_appliance;
                event_count += 1;
                if duration == 0. {
                    continue;
                }
                // step will depend on timestep of flatprofile, always hourly so no adjustment
                eventlist.push(ApplianceGainsEvent {
                    start: step as f64 + start_offset,
                    duration,
                    demand_w: demand_w_event,
                });

                // build the flattened profile for use with loadshifting
                let mut integralx: f64 = Default::default();
                while integralx < duration {
                    let segment = (start_offset.ceil() - start_offset).min(duration - integralx);
                    sched[(step + (start_offset + integralx).floor() as usize)
                        % flat_profile_len] += (demand_w_event - standby_w) * segment;
                    integralx += segment;
                }
                start_offset += e as f64 * duration;
            }
        }

        Ok((eventlist, sched))
    }
}

const DEFAULT_SEED: usize = 37;
const DEFAULT_DURATION_STD_DEV: f64 = 0.;

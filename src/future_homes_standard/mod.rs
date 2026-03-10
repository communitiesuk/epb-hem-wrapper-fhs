use crate::future_homes_standard::fhs_compliance_response::{
    CalculatedComplianceResult, FhsComplianceResponse,
};
use crate::future_homes_standard::future_homes_standard_notional::apply_fhs_notional_preprocessing;
use crate::future_homes_standard::input::InputForProcessing;
use crate::HemWrapper;
use crate::{CalculationKey, FhsFlags};

use future_homes_standard::{apply_fhs_postprocessing, apply_fhs_preprocessing};
use future_homes_standard_fee::{apply_fhs_fee_postprocessing, apply_fhs_fee_preprocessing};
use home_energy_model::input::CustomEnergySourceFactor;
use home_energy_model::output::{OutputCore, OutputSummary};
use home_energy_model::output_writer::OutputWriter;
use home_energy_model::CalculationResult;
use home_energy_model::HemResponse;
use indexmap::IndexMap;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

mod fhs_appliance;
mod fhs_compliance_response;
mod fhs_hw_events;
mod fhs_imev_scheduler;
mod fhs_part_f_validation;
pub mod fhs_schema_validation;
pub(crate) mod fhs_sleeved_dhn_validation;
pub(crate) mod fhs_storeys_validation;
pub(crate) mod fhs_ventilation;
pub(crate) mod fhs_window_validation;
#[allow(clippy::module_inception)]
pub mod future_homes_standard;
pub(crate) mod future_homes_standard_fee;
pub(crate) mod future_homes_standard_notional;
pub(crate) mod input;
pub(crate) mod metrics;
mod project_lookups;

/// A HEM wrapper for all single calculations using the FHS wrapper.
pub struct FhsSingleCalcWrapper;

impl Default for FhsSingleCalcWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl FhsSingleCalcWrapper {
    pub fn new() -> Self {
        Self {}
    }
}

impl HemWrapper for FhsSingleCalcWrapper {
    fn apply_preprocessing(
        &self,
        mut input: InputForProcessing,
        custom_energy_supply_factors: &IndexMap<Arc<str>, CustomEnergySourceFactor>,
        flags: &FhsFlags,
    ) -> anyhow::Result<HashMap<CalculationKey, InputForProcessing>> {
        do_fhs_preprocessing(&mut input, custom_energy_supply_factors, flags)?;
        Ok(HashMap::from([(CalculationKey::Primary, input)]))
    }

    fn apply_postprocessing(
        &self,
        output: &impl OutputWriter,
        results: &HashMap<CalculationKey, CalculationResult>,
        flags: &FhsFlags,
    ) -> anyhow::Result<Option<HemResponse>> {
        let results = results
            .get(&CalculationKey::Primary)
            .expect("A primary calculation was expected in the FHS single calc wrapper");
        do_fhs_postprocessing(output, results, flags)
    }
}

/// A HEM wrapper for full FHS compliance calculations.
pub struct FhsComplianceWrapper;

impl Default for FhsComplianceWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl FhsComplianceWrapper {
    pub fn new() -> Self {
        Self {}
    }
}

impl HemWrapper for FhsComplianceWrapper {
    fn apply_preprocessing(
        &self,
        input: InputForProcessing,
        custom_energy_supply_factors: &IndexMap<Arc<str>, CustomEnergySourceFactor>,
        _flags: &FhsFlags,
    ) -> anyhow::Result<HashMap<CalculationKey, InputForProcessing>> {
        vec![input; FHS_COMPLIANCE_CALCULATIONS.len()]
            .into_par_iter()
            .enumerate()
            .map(|(i, mut input)| {
                let (key, flags) = &FHS_COMPLIANCE_CALCULATIONS[i];
                do_fhs_preprocessing(&mut input, custom_energy_supply_factors, flags)?;
                Ok((*key, input))
            })
            .collect::<anyhow::Result<HashMap<CalculationKey, InputForProcessing>>>()
    }

    fn apply_postprocessing(
        &self,
        output: &impl OutputWriter,
        results: &HashMap<CalculationKey, CalculationResult>,
        _flags: &FhsFlags,
    ) -> anyhow::Result<Option<HemResponse>> {
        FHS_COMPLIANCE_CALCULATIONS
            .par_iter()
            .map(|(key, flags)| {
                do_fhs_postprocessing(output, &results[key], flags)?;
                Ok(())
            })
            .collect::<anyhow::Result<()>>()?;

        let compliance_result = CalculatedComplianceResult::try_from(results)?;
        let compliance_response = FhsComplianceResponse::build_from(&compliance_result)?;

        Ok(Some(HemResponse::new(compliance_response)))
    }
}

static FHS_COMPLIANCE_CALCULATIONS: LazyLock<[(CalculationKey, FhsFlags); 4]> =
    LazyLock::new(|| {
        [
            (CalculationKey::Fhs, FhsFlags::FHS_ASSUMPTIONS),
            (CalculationKey::FhsFee, FhsFlags::FHS_FEE_ASSUMPTIONS),
            (
                CalculationKey::FhsNotional,
                FhsFlags::FHS_NOT_A_ASSUMPTIONS | FhsFlags::FHS_NOT_B_ASSUMPTIONS,
            ),
            (
                CalculationKey::FhsNotionalFee,
                FhsFlags::FHS_FEE_NOT_A_ASSUMPTIONS | FhsFlags::FHS_FEE_NOT_B_ASSUMPTIONS,
            ),
        ]
    });

fn do_fhs_preprocessing(
    input_for_processing: &mut InputForProcessing,
    custom_energy_supply_factors: &IndexMap<Arc<str>, CustomEnergySourceFactor>,
    flags: &FhsFlags,
) -> anyhow::Result<()> {
    // Apply required preprocessing steps, if any
    if flags.intersects(
        FhsFlags::FHS_NOT_A_ASSUMPTIONS
            | FhsFlags::FHS_NOT_B_ASSUMPTIONS
            | FhsFlags::FHS_FEE_NOT_A_ASSUMPTIONS
            | FhsFlags::FHS_FEE_NOT_B_ASSUMPTIONS,
    ) {
        apply_fhs_notional_preprocessing(
            input_for_processing,
            custom_energy_supply_factors,
            false,
        )?;
    }
    if flags.intersects(
        FhsFlags::FHS_ASSUMPTIONS
            | FhsFlags::FHS_NOT_A_ASSUMPTIONS
            | FhsFlags::FHS_NOT_B_ASSUMPTIONS,
    ) {
        apply_fhs_preprocessing(input_for_processing, Some(false), None)?;
    } else if flags.intersects(
        FhsFlags::FHS_FEE_ASSUMPTIONS
            | FhsFlags::FHS_FEE_NOT_A_ASSUMPTIONS
            | FhsFlags::FHS_FEE_NOT_B_ASSUMPTIONS,
    ) {
        apply_fhs_fee_preprocessing(input_for_processing)?;
    }

    Ok(())
}

fn do_fhs_postprocessing(
    output_writer: &impl OutputWriter,
    results: &CalculationResult,
    flags: &FhsFlags,
) -> anyhow::Result<Option<HemResponse>> {
    let input = &results.input.clone();
    let OutputCore {
        timestep_array,
        results_end_user,
        energy_import,
        energy_export,
        ..
    } = &results.output.core;

    if flags.intersects(
        FhsFlags::FHS_ASSUMPTIONS
            | FhsFlags::FHS_NOT_A_ASSUMPTIONS
            | FhsFlags::FHS_NOT_B_ASSUMPTIONS,
    ) {
        let notional =
            flags.intersects(FhsFlags::FHS_NOT_A_ASSUMPTIONS | FhsFlags::FHS_NOT_B_ASSUMPTIONS);
        apply_fhs_postprocessing(
            input,
            output_writer,
            energy_import,
            energy_export,
            results_end_user,
            timestep_array,
            notional,
        )?;
    } else if flags.intersects(
        FhsFlags::FHS_FEE_ASSUMPTIONS
            | FhsFlags::FHS_FEE_NOT_A_ASSUMPTIONS
            | FhsFlags::FHS_FEE_NOT_B_ASSUMPTIONS,
    ) {
        let OutputSummary {
            space_heat_demand_total,
            space_cool_demand_total,
            total_floor_area,
            ..
        } = results.output.summary;
        apply_fhs_fee_postprocessing(
            output_writer,
            total_floor_area,
            space_heat_demand_total,
            space_cool_demand_total,
        )?;
    }

    Ok(None)
}

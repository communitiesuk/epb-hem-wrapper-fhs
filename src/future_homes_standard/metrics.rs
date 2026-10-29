#![allow(unused)]
use crate::future_homes_standard::project_lookups::{FuelOutput, Tariff};
use arcstr::ArcStr;
use fsum::FSum;
use serde::Serialize;
use std::sync::Arc;
use strum_macros::Display;

#[derive(Display, Copy, Clone, Debug, PartialEq, Serialize)]
enum Grade {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}
const EER_GRADE_CUTOFFS: [(Grade, f64); 7] = [
    (Grade::G, 20.),
    (Grade::F, 38.),
    (Grade::E, 54.),
    (Grade::D, 68.),
    (Grade::C, 80.),
    (Grade::B, 91.),
    (Grade::A, f64::INFINITY),
];

const ENERGY_COST_DEFLATOR: f64 = 0.36;
const PENCE_PER_POUND: f64 = 100.;

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct Metric {
    description: Arc<str>,
    grade: Option<Grade>,
    units: Arc<str>,
    value: Option<f64>,
}

#[derive(Serialize)]
pub(crate) struct Metrics {
    pub(crate) eer: Metric,
}

pub(crate) fn energy_efficiency_rating(total_floor_area: f64, by_fuel: &[FuelOutput]) -> Metric {
    // Do not provide EER if any tariff tier does not have unit or standing charge implemented
    if by_fuel
        .iter()
        .flat_map(|FuelOutput { tariffs, .. }| tariffs.iter())
        .any(
            |Tariff {
                 standing_charge,
                 unit_price,
                 ..
             }| { standing_charge.is_none() || unit_price.is_none() },
        )
    {
        return Metric {
            description: "Energy efficiency rating (EER) not applicable for custom fuels".into(),
            units: "Normalised cost, typically in range 0-100".into(),
            value: None,
            grade: None,
        };
    }

    let total_cost = FSum::with_all(
        by_fuel
            .iter()
            .flat_map(|fuel_output| fuel_output.tariffs.iter())
            .flat_map(
                |Tariff {
                     standing_charge,
                     unit_price,
                     eer_energy,
                 }| {
                    if let (Some(standing_charge), Some(unit_price)) = (standing_charge, unit_price)
                    {
                        Some(*standing_charge + *unit_price / PENCE_PER_POUND * *eer_energy)
                    } else {
                        None
                    }
                },
            ),
    )
    .value();

    let energy_cost_factor = ENERGY_COST_DEFLATOR * total_cost / (total_floor_area + 45.);
    let eer = if energy_cost_factor >= 3.5 {
        108.8 - 120.5 * energy_cost_factor.log10()
    } else {
        100. - 16.21 * energy_cost_factor
    };
    let eer = eer.max(1.0);

    Metric {
        description: "Energy efficiency rating based on total energy costs for space/water heating, ventilation and lighting".into(),
        units: "Normalised cost, typically in range 0-100".into(),
        value: Some(eer),
        grade: Some(energy_efficiency_grade(eer)),
    }
}

fn energy_efficiency_grade(eer: f64) -> Grade {
    EER_GRADE_CUTOFFS
        .iter()
        .find_map(|(grade, cutoff)| (eer <= *cutoff).then_some(*grade))
        .unwrap_or(Grade::A)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Given EER applicable energy, disaggregated by fuel
    fn test_energy_efficiency_rating() {
        let by_fuel = [
            FuelOutput {
                fuel: "electricity".into(),
                tariffs: vec![Tariff {
                    eer_energy: 1000.,
                    unit_price: Some(16.49),
                    standing_charge: Some(0.0),
                }],
            },
            FuelOutput {
                fuel: "the power of love".into(),
                tariffs: vec![Tariff {
                    eer_energy: 100.,
                    unit_price: Some(1.),
                    standing_charge: Some(100.0),
                }],
            },
        ];

        // When the EER metric is calculated
        let eer = energy_efficiency_rating(100., &by_fuel);
        let expected = Metric {
            description: "Energy efficiency rating based on total energy costs for space/water heating, ventilation and lighting".into(),
            units: "Normalised cost, typically in range 0-100".into(),
            value: Some(89.29871696551724),
            grade: Some(Grade::B),
        };

        // Then the metric shows the EER rating and grade of the dwelling
        // cost = 0 + 16.49 / 100 * 1000 + 100 + 1 / 100 * 100 = 265.9
        // ecf = 0.36 * 265.9 / (100 + 45) = 0.660165517
        // eer = 100 - 16.21 * 0.660165517 = 89.298716969
        assert_eq!(eer, expected);
    }

    #[test]
    /// Given EER applicable energy, disaggregated by fuel
    fn test_test_energy_efficiency_rating_below_one() {
        let by_fuel = [
            FuelOutput {
                fuel: "electricity".into(),
                tariffs: vec![Tariff {
                    eer_energy: 100_000.,
                    unit_price: Some(16.49),
                    standing_charge: Some(0.0),
                }],
            },
            FuelOutput {
                fuel: "the power of love".into(),
                tariffs: vec![Tariff {
                    eer_energy: 100.,
                    unit_price: Some(1.),
                    standing_charge: Some(100.0),
                }],
            },
        ];

        // When the EER metric is calculated
        let eer = energy_efficiency_rating(100., &by_fuel);
        // then the metric shows the EER rating rounded to 1
        assert_eq!(eer, Metric {
            description: "Energy efficiency rating based on total energy costs for space/water heating, ventilation and lighting".into(),
            units: "Normalised cost, typically in range 0-100".into(),
            value: Some(1.),
            grade: Grade::G.into(),
        });
    }

    #[test]
    /// Given EER applicable energy, disaggregated by fuel, one of which is "custom"
    fn test_energy_efficiency_rating_with_custom_fuel() {
        let by_fuel = [
            FuelOutput {
                fuel: "electricity".into(),
                tariffs: vec![Tariff {
                    eer_energy: 1000.,
                    unit_price: Some(16.49),
                    standing_charge: Some(0.0),
                }],
            },
            FuelOutput {
                fuel: "custom".into(),
                tariffs: vec![Tariff {
                    eer_energy: 100.,
                    unit_price: None,
                    standing_charge: None,
                }],
            },
        ];

        // When the EER metric is calculated
        let eer = energy_efficiency_rating(100., &by_fuel);
        let expected = Metric {
            description: "Energy efficiency rating (EER) not applicable for custom fuels".into(),
            units: "Normalised cost, typically in range 0-100".into(),
            value: None,
            grade: None,
        };

        // Then the metric shows the EER as unimplemented
        assert_eq!(eer, expected);
    }

    #[test]
    /// Given EER applicable energy on Economy 7, with off-peak and on-peak tiers
    fn test_energy_efficiency_rating_economy7() {
        let by_fuel = [FuelOutput {
            fuel: "electricity_economy7".into(),
            tariffs: vec![
                Tariff {
                    eer_energy: 1000.,
                    unit_price: Some(9.40),
                    standing_charge: Some(7.0),
                },
                Tariff {
                    eer_energy: 500.,
                    unit_price: Some(19.60),
                    standing_charge: Some(0.0),
                },
            ],
        }];

        // When the EER metric is calculated
        let eer = energy_efficiency_rating(100., &by_fuel);
        // Then both tiers contribute to the total cost
        // cost = 7.0 + 9.40/100 * 1000 + 0.0 + 19.60/100 * 500 = 7 + 94 + 98 = 199.0
        // ecf = 0.36 * 199.0 / (100 + 45) = 71.64 / 145 = 0.49406896551...
        // eer = 100 - 16.21 * 0.49406896551... = 91.99114206896552
        assert_eq!(
            eer,
            Metric {
                description: "Energy efficiency rating based on total energy costs for space/water heating, ventilation and lighting".into(),
                units: "Normalised cost, typically in range 0-100".into(),
                value: Some(91.99114206896552),
                grade: Grade::A.into(),
            }
        );
    }
}

use home_energy_model::input::Vent;
use indexmap::IndexMap;

use crate::future_homes_standard::input::InputForProcessing;

const MIN_KITCHEN_VENT_FLOW_RATE: f64 = 60.0; // the property needs one 60+ l/s fan for cooking events

pub fn minimum_whole_dwelling_ventilation_rate_continuous(
    total_floor_area: f64,
    bedrooms: u32,
) -> f64 {
    todo!();
}

fn minimum_whole_dwelling_ventilation_rate_intermittent(
    bathrooms: u32,
    utility_rooms: u32,
    sanitary_accommodations: u32,
) -> f64 {
    todo!();
}

fn minimum_background_ventilation_area_intermittent(
    habitable_rooms: u32,
    bathrooms: u32,
    storeys: u32,
) -> f64 {
    todo!();
}

fn sufficient_whole_dwelling_ventilation_rate_continuous(
    vents: Vec<IndexMap<String, Vent>>,
    total_floor_area: f64,
    bedrooms: u32,
) -> bool {
    todo!()
}

fn sufficient_whole_dwelling_ventilation_rate_intermittent(
    vents: Vec<IndexMap<String, Vent>>,
    bathrooms: u32,
    utility_rooms: u32,
    sanitary_accommodations: u32,
) -> bool {
    todo!()
}

fn sufficient_background_ventilation_area_continuous(
    vents: Vec<IndexMap<String, Vent>>,
    habitable_rooms: u32,
) -> bool {
    todo!();
}

fn sufficient_background_ventilation_area_intermittent(
    vents: Vec<IndexMap<String, Vent>>,
    habitable_rooms: u32,
    bathrooms: u32,
    storeys: u32,
) -> bool {
    todo!();
}

fn sufficient_imev_count(vents: Vec<IndexMap<String, Vent>>, wet_rooms: u32) -> bool {
    todo!();
}

fn sufficient_large_imev(vents: Vec<IndexMap<String, Vent>>) -> bool {
    todo!();
}

fn validate_dwelling_ventilation(
    ventilation: IndexMap<String, Vent>,
    total_floor_area: f64,
    bedrooms: u32,
    habitable_rooms: u32,
    wet_rooms: u32,
    bathrooms: u32,
    utility_rooms: u32,
    sanitary_accommodations: u32,
    storeys: u32,
) -> () {
    todo!();
}

fn minimum_background_ventilation(project_dict: InputForProcessing) -> () {
    // TODO set return type correctly
    todo!();
}

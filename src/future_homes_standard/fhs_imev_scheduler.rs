enum EventType {
    Tapping,
    Cooking,
}
struct Event {
    start: f64,
    duration: f64,
    event_type: Option<EventType>,
}

#[derive(Debug, PartialEq, Default, Clone)]
struct Time {
    start: f64,
    end: f64,
}
impl Event {
    fn chunkify(&self, simulation_start_time: usize, simulation_end_time: usize) -> Vec<Time> {
        // if duration takes us beyond the simulation time or the start is < simulation start time
        // break into multiple events strictly starting and ending during the simulation
        // with start < end
        let simulation_start_time = simulation_start_time as f64;
        let simulation_end_time = simulation_end_time as f64;

        let times = if self.start < simulation_start_time {
            let underspill = simulation_start_time - self.start;
            vec![
                Time {
                    start: simulation_end_time - underspill,
                    end: simulation_end_time,
                },
                Time {
                    start: simulation_start_time,
                    end: simulation_start_time + self.duration - underspill,
                },
            ]
        } else {
            todo!()
        };

        times
    }
}

#[cfg(test)]
mod test {
    mod test_event {
        use crate::future_homes_standard::fhs_imev_scheduler::{Event, Time};

        #[test]
        fn test_event_chunkifies_start_time_less_than_start_time() {
            // Given an event that starts before t=0
            let event = Event {
                start: -1.,
                duration: 5.,
                event_type: None,
            };
            // When the event is chunkified
            let times = event.chunkify(0, 10);
            // Then the event is split into two time periods
            assert_eq!(
                times,
                &[
                    Time {
                        start: 9.,
                        end: 10.
                    },
                    Time { start: 0., end: 4. }
                ]
            );
        }
    }
}

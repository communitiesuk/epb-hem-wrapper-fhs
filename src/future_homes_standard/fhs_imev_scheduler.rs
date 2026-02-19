use anyhow::bail;

enum EventType {
    Tapping,
    Cooking,
}
struct Event {
    start: f64,
    duration: f64,
    event_type: Option<EventType>,
}

#[derive(Debug, PartialEq)]
struct Time {
    start: f64,
    end: f64,
}
impl Event {
    fn chunkify(
        &self,
        simulation_start_time: f64,
        simulation_end_time: f64,
    ) -> anyhow::Result<Vec<Time>> {
        // if duration takes us beyond the simulation time or the start is < simulation start time
        // break into multiple events strictly starting and ending during the simulation
        // with start < end
        if self.start + self.duration <= simulation_start_time || self.start >= simulation_end_time
        {
            bail!(
                "Event (start={}, duration={}) wholly outside the simulation time ({} to {})",
                self.start,
                self.duration,
                simulation_start_time,
                simulation_end_time
            );
        }
        if self.duration > simulation_end_time - simulation_start_time {
            bail!(
                "Event (start={}, duration={}) is longer than the simulation time ({} to {})",
                self.start,
                self.duration,
                simulation_start_time,
                simulation_end_time
            );
        }

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
        } else if self.start + self.duration > simulation_end_time {
            let overspill = self.start + self.duration - simulation_end_time;
            vec![
                Time {
                    start: self.start,
                    end: self.start + self.duration - overspill,
                },
                Time {
                    start: simulation_start_time,
                    end: simulation_start_time + overspill,
                },
            ]
        } else {
            vec![Time {
                start: self.start,
                end: self.start + self.duration,
            }]
        };
        Ok(times)
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
            let times = event.chunkify(0., 10.).unwrap();
            // Then the event is split into two time periods
            assert_eq!(
                times[0],
                Time {
                    start: 9.,
                    end: 10.
                }
            );
            assert_eq!(times[1], Time { start: 0., end: 4. });
        }

        #[test]
        fn test_event_chunkifies_duration_overspilling_loop_time() {
            // Given an event that ends after the end of the time period
            let event = Event {
                start: 8.,
                duration: 5.,
                event_type: None,
            };
            // When the event is chunkified
            let times = event.chunkify(0., 10.).unwrap();
            // Then the event is split into two time periods
            assert_eq!(
                times[0],
                Time {
                    start: 8.,
                    end: 10.
                }
            );
            assert_eq!(times[1], Time { start: 0., end: 3. });
        }

        #[test]
        fn test_event_chunkifies_covering_both_ends_of_time_window() {
            // Given an event that ends after the end of the time period
            let event = Event {
                start: -1.,
                duration: 12.,
                event_type: None,
            };
            // The event can't be chunkified
            let times = event.chunkify(0., 10.);
            assert!(times.is_err());
            // So appropriate error
            let errror = times.unwrap_err().to_string();
            assert_eq!(
                errror,
                "Event (start=-1, duration=12) is longer than the simulation time (0 to 10)"
            );
        }

        #[test]
        fn test_event_falling_within_loop_time() {
            // Given an event that ends after the end of the time period
            let event = Event {
                start: 1.,
                duration: 5.,
                event_type: None,
            };
            // When the event is chunkified
            let times = event.chunkify(0., 10.).unwrap();
            // Then the event is converted to a single time
            assert_eq!(times[0], Time { start: 1., end: 6. });
        }

        #[test]
        fn test_error_for_event_before_time_window() {
            // Given an event that ends before the start of the time period
            let event = Event {
                start: -10.,
                duration: 5.,
                event_type: None,
            };
            let times = event.chunkify(0., 10.);
            // The event can't be chunkified
            assert!(times.is_err());
            // So appropriate error
            assert_eq!(
                times.unwrap_err().to_string(),
                "Event (start=-10, duration=5) wholly outside the simulation time (0 to 10)"
            );
        }

        #[test]
        fn test_error_for_event_after_time_window() {
            // Given an event that ends before the start of the time period
            let event = Event {
                start: 15.,
                duration: 5.,
                event_type: None,
            };
            let times = event.chunkify(0., 10.);
            // The event can't be chunkified
            assert!(times.is_err());
            // So appropriate error
            assert_eq!(
                times.unwrap_err().to_string(),
                "Event (start=15, duration=5) wholly outside the simulation time (0 to 10)"
            );
        }

        #[test]
        fn test_event_chunkifies_start_time_less_than_start_time_non_zero_start_time() {
            // Given an event that starts before t=0
            let event = Event {
                start: 0.,
                duration: 5.,
                event_type: None,
            };
            // When the event is chunkified
            let times = event.chunkify(1., 11.).unwrap();
            // Then the event is split into two time periods
            assert_eq!(
                times[0],
                Time {
                    start: 10.,
                    end: 11.
                }
            );
            assert_eq!(times[1], Time { start: 1., end: 5. });
        }
    }
}

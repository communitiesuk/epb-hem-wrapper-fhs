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

struct Imev {
    name: String,
    on_times: Vec<Time>,
    simulation_start_time: i32,
    simulation_end_time: i32,
    timestep: f64,
}

impl Imev {
    fn new(
        name: &str,
        simulation_start_time: i32,
        simulation_end_time: i32,
        timestep: f64,
    ) -> Self {
        Self {
            name: name.into(),
            on_times: Default::default(),
            simulation_start_time,
            simulation_end_time,
            timestep,
        }
    }

    fn set_on_times(&mut self, on_times: Vec<Time>) {
        self.on_times = on_times;
    }

    fn on_fraction(&self, event: Event) -> anyhow::Result<f64> {
        // fraction of the given event's duration that the iMEV is on
        let test_periods = event.chunkify(
            self.simulation_start_time as f64,
            self.simulation_end_time as f64,
        )?;
        let total_duration: f64 = test_periods.iter().map(|p| p.end - p.start).sum::<f64>();
        let mut duration_on = 0.;
        for period in test_periods {
            let mut on_times = self
                .on_times
                .iter()
                .filter(|on| !(on.start > period.end || on.end < period.start))
                .collect::<Vec<&Time>>();
            let mut current_time_end = period.start;
            while current_time_end < period.end && !on_times.is_empty() {
                let current_time = on_times
                    .iter()
                    .min_by(|a, b| a.start.partial_cmp(&b.start).unwrap())
                    .unwrap();
                duration_on +=
                    (period.end.min(current_time.end)) - (current_time_end.max(current_time.start));
                current_time_end = current_time.end;
                // Keep only on_times that extend beyond current_time_end
                on_times.retain(|on| on.end > current_time_end);
            }
        }
        Ok(duration_on / total_duration)
    }
    
    fn schedulise(&self) -> anyhow::Result<Vec<f64>> {
        let mut result = Vec::new();
        for i in self.simulation_start_time
            ..(self.simulation_end_time as f64 / self.timestep).round() as i32
        {
            result.push(self.on_fraction(Event {
                start: i as f64 * self.timestep,
                duration: self.timestep,
                event_type: None,
            })?);
        }
        Ok(result)
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

    mod test_imev_schedulise {
        use crate::future_homes_standard::fhs_imev_scheduler::{Imev, Time};

        #[test]
        fn test_returns_on_fractions_for_non_overlapping_on_times() {
            // Given an IMEV with on_times within a given time window
            let mut imev = Imev::new("venty mcventface", 2, 5, 1.);
            imev.set_on_times(vec![
                Time { start: 2., end: 3. },
                Time {
                    start: 3.5,
                    end: 4.5,
                },
            ]);
            // When converting to a schedule
            let schedule = imev.schedulise().unwrap();
            // Then the schedule reflects the time that the vent is on in each timestep
            assert_eq!(schedule, vec![1., 0.5, 0.5]);
        }
    }
}

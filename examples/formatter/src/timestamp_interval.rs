use resol_vbus::chrono::{DateTime, Duration, Local};

pub struct TimestampInterval {
    interval: Option<Duration>,
    last_interval: Option<i64>,
}

impl TimestampInterval {
    pub fn new(interval: Option<Duration>) -> TimestampInterval {
        TimestampInterval {
            interval: interval,
            last_interval: None,
        }
    }

    pub fn is_new_interval(&mut self, timestamp: &DateTime<Local>) -> bool {
        if let Some(interval) = self.interval {
            let current_interval = timestamp.naive_local().timestamp() / interval.num_seconds();

            let new_interval = match self.last_interval {
                Some(last_interval) => current_interval != last_interval,
                None => true,
            };

            self.last_interval = Some(current_interval);

            new_interval
        } else {
            true
        }
    }
}

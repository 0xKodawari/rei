use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;
use std::time::Duration;

// Todo
// 1. How to handle date changes and clean up between potential calls

// --------------------------------- Functions and structures for tracking and keeping track of immersion ---------------------------------

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct DailyImmersion {
    // Start of the most recent interval of immersion
    pub set: DateTime<Utc>,
    // Total Immersion on the day
    pub total: Duration,
    // Is this already being tracked or not
    pub active: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Goal {
    // Daily desired amount spent on the goal
    pub daily: Duration,
    // Amount spent within the current day and historical information
    pub current: HashMap<NaiveDate, DailyImmersion>,
    // Total amount spent on the goal since tracking
    pub total: Duration,
}

impl Goal {
    pub fn stop(&mut self) {
        let date = Utc::now();
        let day = date.date_naive();
        self.current
            .entry(day)
            .and_modify(|time| {
                if time.active {
                    let delta = date - time.set;
                    let dur =
                        Duration::new(delta.num_seconds() as u64, delta.subsec_nanos() as u32);
                    // Have something in place for if this panics with nanoseconds
                    time.total += dur;
                    self.total += dur;
                    time.active = false;
                }
            })
            .or_default();
    }

    pub fn start(&mut self) {
        let date = Utc::now();
        let day = date.date_naive();
        self.current
            .entry(day)
            .and_modify(|time| {
                if !time.active {
                    time.set = date;
                    time.active = true;
                }
            })
            .or_insert(DailyImmersion {
                set: date,
                total: Duration::ZERO,
                active: true,
            });
    }
}

#[derive(Debug, Clone)]
pub struct Immersion {
    pub listening: Goal,
    pub reading: Goal,
}

impl Immersion {
    pub fn new() -> Immersion {
        Immersion {
            listening: Goal {
                daily: Duration::ZERO,
                current: HashMap::new(),
                total: Duration::ZERO,
            },
            reading: Goal {
                daily: Duration::ZERO,
                current: HashMap::new(),
                total: Duration::ZERO,
            },
        }
    }
}

// --------------------------------- Functions and structures for level up system and display ---------------------------------

pub struct User {
    // Name of the user profile
    pub name: String,
    // Immersion Data
    pub immersion: Immersion,
    // Stats for the level system and display
    pub stats: u64,
    // Last day the user was online
    pub last_login: NaiveDate,
}

impl User {
    pub fn create(name: String) -> User {
        User {
            name,
            immersion: Immersion::new(),
            stats: 0,
            last_login: Utc::now().date_naive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;

    use super::Immersion;

    #[test]
    fn test_track() {
        let date = Utc::now().date_naive();
        let mut immersion = Immersion::new();

        immersion.listening.start();
        let current = immersion.listening.current.get(&date);
        assert!(current.is_some());
        let current = current.unwrap();
        assert!(current.active);

        std::thread::sleep(Duration::new(1, 0));

        let mut clone = immersion.clone();
        clone.listening.start();
        let after_sleep_current = clone.listening.current.get(&date).unwrap();
        assert_eq!(current, after_sleep_current);

        immersion.listening.stop();
        let updated_current = immersion.listening.current.get(&date).unwrap();

        assert!(!updated_current.active);
        assert!(!updated_current.total.is_zero());
        assert!(!immersion.listening.total.is_zero());
        assert_ne!(updated_current, after_sleep_current);
    }
}

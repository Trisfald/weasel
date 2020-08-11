//! Metrics for battles.

use crate::battle::BattleRules;
use crate::error::{WeaselError, WeaselResult};
use crate::user::{UserMetricId, UserRules};
use std::collections::HashMap;
use std::hash::Hash;

/// Manages all metrics in a battle.
pub(crate) struct Metrics<R: BattleRules> {
    map: HashMap<MetricIdType<R>, Metric>,
}

impl<R: BattleRules> Metrics<R> {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Returns a handle to read metrics.
    pub(crate) fn read_handle(&self) -> ReadMetrics<R> {
        ReadMetrics { metrics: self }
    }

    /// Returns a handle to write metrics.
    pub(crate) fn write_handle(&mut self) -> WriteMetrics<R> {
        WriteMetrics { metrics: self }
    }
}

/// Alias for system metrics id.
pub type SystemMetricId = u16;

/// Alias for `MetricId` parameterized on the `BattleRules` R.
pub type MetricIdType<R> = MetricId<<<R as BattleRules>::UR as UserRules<R>>::UserMetricId>;

/// An id to uniquely identify metrics.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MetricId<T> {
    /// System metric.
    System(SystemMetricId),
    /// User defined metric.
    User(T),
}

/// A metric is a compact measurement of some quantity.
#[derive(Copy, Clone)]
pub enum Metric {
    /// A 64 bit unsigned counter.
    CounterU64(u64),
    /// A 64 bit signed counter.
    CounterI64(i64),
    /// A 64 bit floating point counter.
    CounterF64(f64),
}

/// Handle to read metrics.
pub struct ReadMetrics<'a, R: BattleRules> {
    metrics: &'a Metrics<R>,
}

macro_rules! get_metric {
    ($map: expr, $id: expr, $class: ident, $field: ident) => {{
        $map.get(&MetricIdType::<R>::$class($id))
            .and_then(|metric| match metric {
                Metric::$field(v) => Some(*v),
                _ => None,
            })
    }};
}

macro_rules! add_metric {
    ($map: expr, $id: expr, $value: expr, $class: ident, $field: ident) => {{
        let full_id = MetricIdType::<R>::$class($id);
        if let Some(metric) = $map.get_mut(&full_id) {
            match metric {
                Metric::$field(v) => {
                    *v += $value;
                    Ok(())
                }
                _ => Err(WeaselError::WrongMetricType(full_id)),
            }
        } else {
            $map.insert(full_id, Metric::$field($value));
            Ok(())
        }
    }};
}

impl<'a, R: BattleRules> ReadMetrics<'a, R> {
    /// Returns the value of a `u64` system counter.
    ///
    /// Returns `None` if there's no such system counter or if it has another type.
    pub fn system_u64(&self, id: SystemMetricId) -> Option<u64> {
        get_metric!(self.metrics.map, id, System, CounterU64)
    }

    /// Returns the value of a `i64` system counter.
    ///
    /// Returns `None` if there's no such system counter or if it has another type.
    pub fn system_i64(&self, id: SystemMetricId) -> Option<i64> {
        get_metric!(self.metrics.map, id, System, CounterI64)
    }

    /// Returns the value of a `f64` system counter.
    ///
    /// Returns `None` if there's no such system counter or if it has another type.
    pub fn system_f64(&self, id: SystemMetricId) -> Option<f64> {
        get_metric!(self.metrics.map, id, System, CounterF64)
    }

    /// Returns the value of a `u64` user counter.
    ///
    /// Returns `None` if there's no such user counter or if it has another type.
    pub fn user_u64(&self, id: UserMetricId<R>) -> Option<u64> {
        get_metric!(self.metrics.map, id, User, CounterU64)
    }

    /// Returns the value of a `i64` user counter.
    ///
    /// Returns `None` if there's no such user counter or if it has another type.
    pub fn user_i64(&self, id: UserMetricId<R>) -> Option<i64> {
        get_metric!(self.metrics.map, id, User, CounterI64)
    }

    /// Returns the value of a `f64` user counter.
    ///
    /// Returns `None` if there's no such user counter or if it has another type.
    pub fn user_f64(&self, id: UserMetricId<R>) -> Option<f64> {
        get_metric!(self.metrics.map, id, User, CounterF64)
    }
}

/// Handle to write metrics.
pub struct WriteMetrics<'a, R: BattleRules> {
    metrics: &'a mut Metrics<R>,
}

impl<'a, R: BattleRules> WriteMetrics<'a, R> {
    /// Removes a system metric.
    #[allow(dead_code)]
    pub(crate) fn remove_system(&mut self, id: SystemMetricId) {
        self.metrics.map.remove(&MetricIdType::<R>::System(id));
    }

    /// Removes an user metric.
    pub fn remove_user(&mut self, id: UserMetricId<R>) {
        self.metrics.map.remove(&MetricIdType::<R>::User(id));
    }

    /// Adds `value` to the system metric with the given `id`.\
    /// Creates the metric (initialized with `value`) if it doesn't exist.
    ///
    /// Returns an error if the metric exists, but its type is different.
    #[allow(dead_code)]
    pub(crate) fn add_system_u64(&mut self, id: SystemMetricId, value: u64) -> WeaselResult<(), R> {
        add_metric!(self.metrics.map, id, value, System, CounterU64)
    }

    /// Adds `value` to the system metric with the given `id`.\
    ///
    /// Creates the metric (initialized with `value`) if it doesn't exist.
    /// Returns an error if the metric exists, but its type is different.
    #[allow(dead_code)]
    pub(crate) fn add_system_i64(&mut self, id: SystemMetricId, value: i64) -> WeaselResult<(), R> {
        add_metric!(self.metrics.map, id, value, System, CounterI64)
    }

    /// Adds `value` to the system metric with the given `id`.\
    ///
    /// Creates the metric (initialized with `value`) if it doesn't exist.
    /// Returns an error if the metric exists, but its type is different.
    #[allow(dead_code)]
    pub(crate) fn add_system_f64(&mut self, id: SystemMetricId, value: f64) -> WeaselResult<(), R> {
        add_metric!(self.metrics.map, id, value, System, CounterF64)
    }

    /// Adds `value` to the user metric with the given `id`.\
    ///
    /// Creates the metric (initialized with `value`) if it doesn't exist.
    /// Returns an error if the metric exists, but its type is different.
    pub fn add_user_u64(&mut self, id: UserMetricId<R>, value: u64) -> WeaselResult<(), R> {
        add_metric!(self.metrics.map, id, value, User, CounterU64)
    }

    /// Adds `value` to the user metric with the given `id`.\
    ///
    /// Creates the metric (initialized with `value`) if it doesn't exist.
    /// Returns an error if the metric exists, but its type is different.
    pub fn add_user_i64(&mut self, id: UserMetricId<R>, value: i64) -> WeaselResult<(), R> {
        add_metric!(self.metrics.map, id, value, User, CounterI64)
    }

    /// Adds `value` to the user metric with the given `id`.\
    ///
    /// Creates the metric (initialized with `value`) if it doesn't exist.
    /// Returns an error if the metric exists, but its type is different.
    pub fn add_user_f64(&mut self, id: UserMetricId<R>, value: f64) -> WeaselResult<(), R> {
        add_metric!(self.metrics.map, id, value, User, CounterF64)
    }
}

pub mod system {
    //! Contains the id of all system metrics.
    use super::*;

    /// Number of creatures created.
    pub const CREATURES_CREATED: SystemMetricId = 0;
    /// Number of objects created.
    pub const OBJECTS_CREATED: SystemMetricId = 1;
    /// Number of teams created.
    pub const TEAMS_CREATED: SystemMetricId = 2;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::tests::server;
    use crate::{battle_rules, rules::empty::*};

    battle_rules! {}

    #[test]
    fn operations() {
        let mut server = server(CustomRules::new());
        // Write and read all types of metrics.
        let mut writer = server.battle.metrics.write_handle();
        assert_eq!(writer.add_user_u64(0, 4).err(), None);
        assert_eq!(writer.add_user_i64(1, -4).err(), None);
        assert_eq!(writer.add_user_i64(1, -2).err(), None);
        assert_eq!(writer.add_user_f64(2, 5.5).err(), None);
        assert_eq!(writer.add_system_u64(0, 4).err(), None);
        assert_eq!(writer.add_system_i64(1, -4).err(), None);
        assert_eq!(writer.add_system_i64(1, -2).err(), None);
        assert_eq!(writer.add_system_f64(2, 5.5).err(), None);
        let reader = server.battle.metrics.read_handle();
        assert_eq!(reader.user_u64(0), Some(4));
        assert_eq!(reader.user_i64(1), Some(-6));
        assert_eq!(reader.user_f64(2), Some(5.5));
        assert_eq!(reader.system_u64(0), Some(4));
        assert_eq!(reader.system_i64(1), Some(-6));
        assert_eq!(reader.system_f64(2), Some(5.5));
        // Try remove.
        let mut writer = server.battle.metrics.write_handle();
        writer.remove_user(2);
        writer.remove_system(2);
        let reader = server.battle.metrics.read_handle();
        assert_eq!(reader.user_f64(2), None);
        assert_eq!(reader.system_f64(2), None);
    }

    #[test]
    fn error_conditions() {
        let mut server = server(CustomRules::new());
        let mut writer = server.battle.metrics.write_handle();
        assert_eq!(writer.add_user_u64(0, 4).err(), None);
        let reader = server.battle.metrics.read_handle();
        assert_eq!(reader.user_u64(0), Some(4));
        // Check for missing id.
        assert_eq!(reader.user_u64(464), None);
        // Check for wrong system or user call.
        assert_eq!(reader.system_u64(0), None);
        // Check for wrong metric type.
        let mut writer = server.battle.metrics.write_handle();
        assert_eq!(
            writer.add_user_f64(0, 4.4).err(),
            Some(WeaselError::WrongMetricType(MetricId::User(0)))
        );
    }
}

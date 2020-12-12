#![allow(dead_code)]
//! Simple rules engine that represents requirements as a tree, with each node having one or more requirements in order to be "Met".
//!
//! A tree of rules is constructed, and then the `.check_json()` method is called.
//! `json` is a nested `field: value` that will be given to each node in the tree for testing.
//!
//! Status output can be either `Met`, `NotMet`, or `Unknown` if the tested field is not present in the json.
//!
//! To construct a tree, see the following methods.
//!
//! ## Example
//!
//! ```rust
//! extern crate json_rules_engine;
//! use serde_json::json;
//!
//! let tree = json_rules_engine::and(vec![
//!     json_rules_engine::string_equals("name", "John Doe"),
//!     json_rules_engine::or(vec![
//!         json_rules_engine::int_equals("fav_number", 5),
//!         json_rules_engine::int_in_range("thinking_of", 5, 10)
//!     ])
//! ]);
//! let mut facts = json!({
//!     "name": "John Doe",
//!     "fav_number": 5
//! });
//! let result = tree.check_value(&facts);
//! println!("{:?}", result);
//! assert!(result.status == json_rules_engine::Status::Met);
//! // result = ConditionResult { name: "And", status: Met, children: [ConditionResult { name: "Name is John Doe", status: Met, children: [] }, ConditionResult { name: "Or", status: Met, children: [ConditionResult { name: "Favorite number is 5", status: Met, children: [] }, ConditionResult { name: "Thinking of a number between 5 and 10", status: Unknown, children: [] }] }] }
//! ```
//!
//! This creates a tree like the following:
//!
//! ```text
//!                              +---------+
//!                              |   AND   |
//!                              +---------+
//!           _____________________/\_______________
//!          |                                      |
//!          V                                      V
//! +-------------------+                       +--------+
//! | Name is John Doe  |                       |   OR   |
//! +-------------------+                       +--------+
//! | field: "name"     |             ______________/\___________
//! | value: "John Doe" |            |                           |
//! +-------------------+            V                           V
//!                       +----------------------+  +-------------------------+
//!                       | Favorite number is 5 |  | Number between 5 and 10 |
//!                       +----------------------+  +-------------------------+
//!                       | field: "fav_number"  |  | field: "thinking_of"    |
//!                       | value: 5             |  | start: 5                |
//!                       +----------------------+  | end: 10                 |
//!                                                 +-------------------------+
//! ```
//!
//! [1]: enum.Rule.html#method.check

mod core;
mod error;

pub use crate::core::{
    Condition, ConditionResult, Constraint, Engine, Event, EventParams, Rule,
    RuleResult, Status,
};

#[cfg(feature = "eval")]
pub use rhai::{serde::from_dynamic, Map};

pub use error::{Error, Result};

/// Creates a `Rule` where all child `Rule`s must be `Met`
///
/// * If any are `NotMet`, the result will be `NotMet`
/// * If the results contain only `Met` and `Unknown`, the result will be `Unknown`
/// * Only results in `Met` if all children are `Met`
pub fn and(and: Vec<Condition>) -> Condition {
    Condition::And { and }
}

/// Creates a `Rule` where any child `Rule` must be `Met`
///
/// * If any are `Met`, the result will be `Met`
/// * If the results contain only `NotMet` and `Unknown`, the result will be `Unknown`
/// * Only results in `NotMet` if all children are `NotMet`
pub fn or(or: Vec<Condition>) -> Condition {
    Condition::Or { or }
}

/// Creates a `Rule` where `n` child `Rule`s must be `Met`
///
/// * If `>= n` are `Met`, the result will be `Met`, otherwise it'll be `NotMet`
pub fn at_least(
    should_minimum_meet: usize,
    conditions: Vec<Condition>,
) -> Condition {
    Condition::AtLeast {
        should_minimum_meet,
        conditions,
    }
}

/// Creates a rule for string comparison
pub fn string_equals(field: &str, val: &str) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::StringEquals(val.into()),
    }
}

pub fn string_not_equals(field: &str, val: &str) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::StringNotEquals(val.into()),
    }
}

pub fn string_contains(field: &str, val: &str) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::StringContains(val.into()),
    }
}

pub fn string_does_not_contains(field: &str, val: &str) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::StringDoesNotContain(val.into()),
    }
}

pub fn string_in(field: &str, val: Vec<&str>) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::StringIn(
            val.into_iter().map(ToOwned::to_owned).collect(),
        ),
    }
}

pub fn string_not_in(field: &str, val: Vec<&str>) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::StringNotIn(
            val.into_iter().map(ToOwned::to_owned).collect(),
        ),
    }
}

/// Creates a rule for int comparison.
pub fn int_equals(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntEquals(val),
    }
}

pub fn int_not_equals(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntNotEquals(val),
    }
}

pub fn int_contains(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntContains(val),
    }
}

pub fn int_does_not_contain(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntDoesNotContain(val),
    }
}

pub fn int_in(field: &str, val: Vec<i64>) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntIn(val),
    }
}

pub fn int_not_in(field: &str, val: Vec<i64>) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntNotIn(val),
    }
}

pub fn int_in_range(field: &str, start: i64, end: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntInRange(start, end),
    }
}

pub fn int_not_in_range(field: &str, start: i64, end: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntNotInRange(start, end),
    }
}

pub fn int_less_than(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntLessThan(val),
    }
}

pub fn int_less_than_inclusive(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntLessThanInclusive(val),
    }
}

pub fn int_greater_than(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntGreaterThan(val),
    }
}

pub fn int_greater_than_inclusive(field: &str, val: i64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::IntGreaterThanInclusive(val),
    }
}

/// Creates a rule for float comparison.
pub fn float_equals(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatEquals(val),
    }
}

pub fn float_not_equals(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatNotEquals(val),
    }
}

pub fn float_contains(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatContains(val),
    }
}

pub fn float_does_not_contain(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatDoesNotContain(val),
    }
}

pub fn float_in(field: &str, val: Vec<f64>) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatIn(val),
    }
}

pub fn float_not_in(field: &str, val: Vec<f64>) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatNotIn(val),
    }
}

pub fn float_in_range(field: &str, start: f64, end: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatInRange(start, end),
    }
}

pub fn float_not_in_range(field: &str, start: f64, end: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatNotInRange(start, end),
    }
}

pub fn float_less_than(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatLessThan(val),
    }
}

pub fn float_less_than_inclusive(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatLessThanInclusive(val),
    }
}

pub fn float_greater_than(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatGreaterThan(val),
    }
}

pub fn float_greater_than_inclusive(field: &str, val: f64) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::FloatGreaterThanInclusive(val),
    }
}

/// Creates a rule for boolean comparison.
pub fn bool_equals(field: &str, val: bool) -> Condition {
    Condition::Condition {
        field: field.into(),
        constraint: Constraint::BoolEquals(val),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        and, at_least, bool_equals, int_equals, int_in_range, or,
        string_equals, Status,
    };
    use serde_json::{json, Value};

    fn get_test_data() -> Value {
        json!({
            "foo": 1,
            "bar": "bar",
            "baz": true
        })
    }

    #[test]
    fn and_rules() {
        let map = get_test_data();
        // Met & Met == Met
        let mut root =
            and(vec![int_equals("foo", 1), string_equals("bar", "bar")]);
        let mut res = root.check_value(&map);

        assert!(res.status == Status::Met);

        // Met & NotMet == NotMet
        root = and(vec![int_equals("foo", 2), string_equals("bar", "bar")]);
        res = root.check_value(&map);

        assert!(res.status == Status::NotMet);

        // Met & Unknown == Unknown
        root = and(vec![int_equals("quux", 2), string_equals("bar", "bar")]);
        res = root.check_value(&map);

        assert!(res.status == Status::Unknown);

        // NotMet & Unknown == NotMet
        root = and(vec![int_equals("quux", 2), string_equals("bar", "baz")]);
        res = root.check_value(&map);

        assert!(res.status == Status::NotMet);

        // Unknown & Unknown == Unknown
        root = and(vec![int_equals("quux", 2), string_equals("fizz", "bar")]);
        res = root.check_value(&map);

        assert!(res.status == Status::Unknown);
    }

    #[test]
    fn or_rules() {
        let map = get_test_data();
        // Met | Met == Met
        let mut root =
            or(vec![int_equals("foo", 1), string_equals("bar", "bar")]);
        let mut res = root.check_value(&map);

        assert!(res.status == Status::Met);

        // Met | NotMet == Met
        root = or(vec![int_equals("foo", 2), string_equals("bar", "bar")]);
        res = root.check_value(&map);

        assert!(res.status == Status::Met);

        // Met | Unknown == Met
        root = or(vec![int_equals("quux", 2), string_equals("bar", "bar")]);
        res = root.check_value(&map);

        assert!(res.status == Status::Met);

        // NotMet | Unknown == Unknown
        root = or(vec![int_equals("quux", 2), string_equals("bar", "baz")]);
        res = root.check_value(&map);

        assert!(res.status == Status::Unknown);

        // Unknown | Unknown == Unknown
        root = or(vec![int_equals("quux", 2), string_equals("fizz", "bar")]);
        res = root.check_value(&map);

        assert!(res.status == Status::Unknown);
    }

    #[test]
    fn n_of_rules() {
        let map = get_test_data();
        // 2 Met, 1 NotMet == Met
        let mut root = at_least(
            2,
            vec![
                int_equals("foo", 1),
                string_equals("bar", "bar"),
                bool_equals("baz", false),
            ],
        );
        let mut res = root.check_value(&map);

        assert!(res.status == Status::Met);

        // 1 Met, 1 NotMet, 1 Unknown == NotMet
        root = at_least(
            2,
            vec![
                int_equals("foo", 1),
                string_equals("quux", "bar"),
                bool_equals("baz", false),
            ],
        );
        res = root.check_value(&map);

        assert!(res.status == Status::NotMet);

        // 2 NotMet, 1 Unknown == Unknown
        root = at_least(
            2,
            vec![
                int_equals("foo", 2),
                string_equals("quux", "baz"),
                bool_equals("baz", false),
            ],
        );
        res = root.check_value(&map);

        assert!(res.status == Status::NotMet);
    }

    #[test]
    fn string_equals_rule() {
        let map = get_test_data();
        let mut rule = string_equals("bar", "bar");
        let mut res = rule.check_value(&map);
        assert!(res.status == Status::Met);

        rule = string_equals("bar", "baz");
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);
    }

    #[test]
    fn int_equals_rule() {
        let map = get_test_data();
        let mut rule = int_equals("foo", 1);
        let mut res = rule.check_value(&map);
        assert!(res.status == Status::Met);

        rule = int_equals("foo", 2);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);

        // Values not convertible to int should be NotMet
        rule = int_equals("bar", 2);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);
    }

    #[test]
    fn int_range_rule() {
        let map = get_test_data();
        let mut rule = int_in_range("foo", 1, 3);
        let mut res = rule.check_value(&map);
        assert!(res.status == Status::Met);

        rule = int_in_range("foo", 2, 3);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);

        // Values not convertible to int should be NotMet
        rule = int_in_range("bar", 1, 3);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);
    }

    #[test]
    fn boolean_rule() {
        let mut map = get_test_data();
        let mut rule = bool_equals("baz", true);
        let mut res = rule.check_value(&map);
        assert!(res.status == Status::Met);

        rule = bool_equals("baz", false);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);

        rule = bool_equals("bar", true);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);

        rule = bool_equals("bar", false);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);

        map["quux".to_owned()] = json!("tRuE");
        rule = bool_equals("quux", true);
        res = rule.check_value(&map);
        assert!(res.status == Status::NotMet);
    }
}

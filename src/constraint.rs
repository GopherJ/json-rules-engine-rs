use crate::status::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "operator", content = "value")]
pub enum Constraint {
    StringEquals(String),
    StringNotEquals(String),
    StringContains(String),
    StringContainsAll(Vec<String>),
    StringContainsAny(Vec<String>),
    StringDoesNotContain(String),
    StringDoesNotContainAny(Vec<String>),
    StringIn(Vec<String>),
    StringNotIn(Vec<String>),
    IntEquals(i64),
    IntNotEquals(i64),
    IntContains(i64),
    IntContainsAll(Vec<i64>),
    IntContainsAny(Vec<i64>),
    IntDoesNotContain(i64),
    IntDoesNotContainAny(Vec<i64>),
    IntIn(Vec<i64>),
    IntNotIn(Vec<i64>),
    IntInRange(i64, i64),
    IntNotInRange(i64, i64),
    IntLessThan(i64),
    IntLessThanInclusive(i64),
    IntGreaterThan(i64),
    IntGreaterThanInclusive(i64),
    FloatEquals(f64),
    FloatNotEquals(f64),
    FloatContains(f64),
    FloatDoesNotContain(f64),
    FloatIn(Vec<f64>),
    FloatNotIn(Vec<f64>),
    FloatInRange(f64, f64),
    FloatNotInRange(f64, f64),
    FloatLessThan(f64),
    FloatLessThanInclusive(f64),
    FloatGreaterThan(f64),
    FloatGreaterThanInclusive(f64),
    BoolEquals(bool),
}

impl Constraint {
    pub fn check_value(&self, v: &Value) -> Status {
        match *self {
            Constraint::StringEquals(ref s) => {
                if let Some(v) = v.as_str() {
                    if v == s {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringNotEquals(ref s) => {
                if let Some(v) = v.as_str() {
                    if v != s {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringContains(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_str()).collect::<Vec<_>>()
                }) {
                    if v.contains(&s.as_str()) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringContainsAll(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_str()).collect::<Vec<_>>()
                }) {
                    if s.iter().all(|y| v.contains(&y.as_str())) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringContainsAny(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_str()).collect::<Vec<_>>()
                }) {
                    if s.iter().any(|y| v.contains(&y.as_str())) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringDoesNotContain(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_str()).collect::<Vec<_>>()
                }) {
                    if !v.contains(&s.as_str()) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringDoesNotContainAny(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_str()).collect::<Vec<_>>()
                }) {
                    if s.iter().all(|y| !v.contains(&y.as_str())) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringIn(ref ss) => {
                if let Some(v) = v.as_str() {
                    if ss.iter().any(|s| s == v) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::StringNotIn(ref ss) => {
                if let Some(v) = v.as_str() {
                    if ss.iter().all(|s| s != v) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntEquals(num) => {
                if let Some(v) = v.as_i64() {
                    if v == num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntNotEquals(num) => {
                if let Some(v) = v.as_i64() {
                    if v != num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntContains(num) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_i64()).collect::<Vec<_>>()
                }) {
                    if v.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntContainsAll(ref nums) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_i64()).collect::<Vec<_>>()
                }) {
                    if nums.iter().all(|num| v.contains(&num)) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntContainsAny(ref nums) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_i64()).collect::<Vec<_>>()
                }) {
                    if nums.iter().any(|num| v.contains(&num)) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntDoesNotContain(num) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_i64()).collect::<Vec<_>>()
                }) {
                    if !v.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntDoesNotContainAny(ref nums) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_i64()).collect::<Vec<_>>()
                }) {
                    if nums.iter().all(|num| !v.contains(&num)) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntIn(ref nums) => {
                if let Some(v) = v.as_i64() {
                    if nums.iter().any(|&num| num == v) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntNotIn(ref nums) => {
                if let Some(v) = v.as_i64() {
                    if nums.iter().all(|&num| num != v) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntInRange(start, end) => {
                if let Some(v) = v.as_i64() {
                    if start <= v && v <= end {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntNotInRange(start, end) => {
                if let Some(v) = v.as_i64() {
                    if start <= v && v <= end {
                        Status::NotMet
                    } else {
                        Status::Met
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntLessThan(num) => {
                if let Some(v) = v.as_i64() {
                    if v < num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntLessThanInclusive(num) => {
                if let Some(v) = v.as_i64() {
                    if v <= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntGreaterThan(num) => {
                if let Some(v) = v.as_i64() {
                    if v > num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntGreaterThanInclusive(num) => {
                if let Some(v) = v.as_i64() {
                    if v >= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatEquals(num) => {
                if let Some(v) = v.as_f64() {
                    if (v - num).abs() < f64::EPSILON {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatNotEquals(num) => {
                if let Some(v) = v.as_f64() {
                    if (v - num).abs() > f64::EPSILON {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatContains(num) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_f64()).collect::<Vec<_>>()
                }) {
                    if v.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatDoesNotContain(num) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.iter().filter_map(|y| y.as_f64()).collect::<Vec<_>>()
                }) {
                    if !v.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatIn(ref nums) => {
                if let Some(v) = v.as_f64() {
                    if nums.iter().any(|&num| (v - num).abs() < f64::EPSILON) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatNotIn(ref nums) => {
                if let Some(v) = v.as_f64() {
                    if nums.iter().all(|&num| (v - num).abs() > f64::EPSILON) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatInRange(start, end) => {
                if let Some(v) = v.as_f64() {
                    if start <= v && v <= end {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatNotInRange(start, end) => {
                if let Some(v) = v.as_f64() {
                    if start <= v && v <= end {
                        Status::NotMet
                    } else {
                        Status::Met
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatLessThan(num) => {
                if let Some(v) = v.as_f64() {
                    if v < num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatLessThanInclusive(num) => {
                if let Some(v) = v.as_f64() {
                    if v <= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatGreaterThan(num) => {
                if let Some(v) = v.as_f64() {
                    if v > num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatGreaterThanInclusive(num) => {
                if let Some(v) = v.as_f64() {
                    if v >= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::BoolEquals(b) => {
                if let Some(v) = v.as_bool() {
                    if v == b {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
        }
    }
}

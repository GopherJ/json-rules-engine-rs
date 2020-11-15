use crate::error::Result;

use std::ops::{BitAnd, BitOr, Not};

use futures_util::future::try_join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::to_value, Value};

// ***********************************************************************
// STATUS
// **********************************************************************
/// The status of a rule check
#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum Status {
    /// Rule was satisfied
    Met,
    /// Rule was not satisfied
    NotMet,
    /// There was not enough information to evaluate
    Unknown,
}

impl BitAnd for Status {
    type Output = Status;
    fn bitand(self, rhs: Status) -> Status {
        match (self, rhs) {
            (Status::Met, Status::Met) => Status::Met,
            (Status::NotMet, _) | (_, Status::NotMet) => Status::NotMet,
            (_, _) => Status::Unknown,
        }
    }
}

impl BitOr for Status {
    type Output = Status;
    fn bitor(self, rhs: Status) -> Status {
        match (self, rhs) {
            (Status::NotMet, Status::NotMet) => Status::NotMet,
            (Status::Met, _) | (_, Status::Met) => Status::Met,
            (_, _) => Status::Unknown,
        }
    }
}

impl Not for Status {
    type Output = Status;

    fn not(self) -> Self::Output {
        match self {
            Status::Met => Status::NotMet,
            Status::NotMet => Status::Met,
            Status::Unknown => Status::Unknown,
        }
    }
}

// ***********************************************************************
// Rule
// **********************************************************************

/// Representation of a node in the rules tree
///
/// It is unnecessary to interact with this type outside of calling `Rule::check()`,
/// to construct the rules tree use the [convenience functions][1] in the module root.
///
/// [1]: index.html#functions
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    And {
        and: Vec<Condition>,
    },
    Or {
        or: Vec<Condition>,
    },
    AtLeast {
        should_minimum_meet: usize,
        conditions: Vec<Condition>,
    },
    Condition {
        field: String,
        #[serde(flatten)]
        constraint: Constraint,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParams {
    #[serde(rename = "type")]
    ty: String,
    title: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Message(EventParams),
    PostToCallbackUrl {
        callback_url: String,
        #[serde(flatten)]
        params: EventParams,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    conditions: Condition,
    event: Event,
}

impl Rule {
    pub fn check_value(&self, info: &Value) -> RuleResult {
        let condition_result = self.conditions.check_value(info);
        let mut event = self.event.to_owned();

        match event {
            Event::Message(ref mut params) | Event::PostToCallbackUrl { ref mut params, .. } => {
                if let Ok(message) = mustache::compile_str(&params.message)
                    .and_then(|template| template.render_to_string(info))
                {
                    params.message = message;
                }
            }
        };

        RuleResult {
            condition_result,
            event,
        }
    }
}

#[derive(Debug)]
pub struct Engine {
    rules: Vec<Rule>,
    client: Client,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            client: Client::new(),
        }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule)
    }

    pub async fn run<T: Serialize>(&self, facts: &T) -> Result<Vec<RuleResult>> {
        let facts = to_value(facts)?;
        let rule_results: Vec<RuleResult> = self
            .rules
            .iter()
            .map(|rule| rule.check_value(&facts))
            .filter(|rule_result| rule_result.condition_result.status == Status::Met)
            .collect();

        let requests = rule_results
            .iter()
            .filter_map(|rule_result| match rule_result.event {
                Event::PostToCallbackUrl {
                    ref callback_url,
                    ref params,
                } => Some(
                    self.client
                        .post(callback_url)
                        .json(&json!({
                            "event_params": params,
                            "facts": &facts,
                        }))
                        .send(),
                ),
                _ => None,
            });

        try_join_all(requests).await?;

        Ok(rule_results)
    }
}

impl Condition {
    /// Starting at this node, recursively check (depth-first) any child nodes and
    /// aggregate the results
    pub fn check_value(&self, info: &Value) -> ConditionResult {
        match *self {
            Condition::And { ref and } => {
                let mut status = Status::Met;
                let children = and
                    .iter()
                    .map(|c| c.check_value(info))
                    .inspect(|r| status = status & r.status)
                    .collect::<Vec<_>>();

                ConditionResult {
                    name: "And".into(),
                    status,
                    children,
                }
            }
            Condition::Or { ref or } => {
                let mut status = Status::NotMet;
                let children = or
                    .iter()
                    .map(|c| c.check_value(info))
                    .inspect(|r| status = status | r.status)
                    .collect::<Vec<_>>();

                ConditionResult {
                    name: "Or".into(),
                    status,
                    children,
                }
            }
            Condition::AtLeast {
                should_minimum_meet,
                ref conditions,
            } => {
                let mut met_count = 0;
                let children = conditions
                    .iter()
                    .map(|c| c.check_value(info))
                    .inspect(|r| {
                        if r.status == Status::Met {
                            met_count += 1;
                        }
                    })
                    .collect::<Vec<_>>();

                let status = if met_count >= should_minimum_meet {
                    Status::Met
                } else {
                    Status::NotMet
                };

                ConditionResult {
                    name: format!(
                        "At least meet {} of {}",
                        should_minimum_meet,
                        conditions.len()
                    ),
                    status,
                    children,
                }
            }
            Condition::Condition {
                ref field,
                ref constraint,
            } => {
                let pointer = if field.starts_with("/") {
                    field.to_owned()
                } else {
                    format!("/{}", field)
                };

                let status = if let Some(s) = info.pointer(&pointer) {
                    constraint.check_value(s)
                } else {
                    Status::Unknown
                };

                ConditionResult {
                    name: field.to_owned(),
                    status,
                    children: Vec::new(),
                }
            }
        }
    }
}

// ***********************************************************************
// CONSTRAINT
// **********************************************************************
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "snake_case"))]
#[serde(tag = "operator", content = "value")]
pub enum Constraint {
    StringEquals(String),
    StringNotEquals(String),
    StringContains(String),
    StringDoesNotContain(String),
    StringIn(Vec<String>),
    StringNotIn(Vec<String>),
    IntEquals(i64),
    IntNotEquals(i64),
    IntContains(i64),
    IntDoesNotContain(i64),
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
                    x.into_iter()
                        .filter_map(|y| y.as_str())
                        .collect::<Vec<&str>>()
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
            Constraint::StringDoesNotContain(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.into_iter()
                        .filter_map(|y| y.as_str())
                        .collect::<Vec<&str>>()
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
                if let Some(val) = v.as_i64() {
                    if val == num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntNotEquals(num) => {
                if let Some(val) = v.as_i64() {
                    if val != num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntContains(num) => {
                if let Some(val) = v.as_array().map(|x| {
                    x.into_iter()
                        .filter_map(|y| y.as_i64())
                        .collect::<Vec<i64>>()
                }) {
                    if val.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntDoesNotContain(num) => {
                if let Some(val) = v.as_array().map(|x| {
                    x.into_iter()
                        .filter_map(|y| y.as_i64())
                        .collect::<Vec<i64>>()
                }) {
                    if !val.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntIn(ref nums) => {
                if let Some(val) = v.as_i64() {
                    if nums.iter().any(|&num| num == val) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntNotIn(ref nums) => {
                if let Some(val) = v.as_i64() {
                    if nums.iter().all(|&num| num != val) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntInRange(start, end) => {
                if let Some(val) = v.as_i64() {
                    if start <= val && val <= end {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntNotInRange(start, end) => {
                if let Some(val) = v.as_i64() {
                    if start <= val && val <= end {
                        Status::NotMet
                    } else {
                        Status::Met
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntLessThan(num) => {
                if let Some(val) = v.as_i64() {
                    if val < num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntLessThanInclusive(num) => {
                if let Some(val) = v.as_i64() {
                    if val <= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntGreaterThan(num) => {
                if let Some(val) = v.as_i64() {
                    if val > num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::IntGreaterThanInclusive(num) => {
                if let Some(val) = v.as_i64() {
                    if val >= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatEquals(num) => {
                if let Some(val) = v.as_f64() {
                    if val == num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatNotEquals(num) => {
                if let Some(val) = v.as_f64() {
                    if val != num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatContains(num) => {
                if let Some(val) = v.as_array().map(|x| {
                    x.into_iter()
                        .filter_map(|y| y.as_f64())
                        .collect::<Vec<f64>>()
                }) {
                    if val.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatDoesNotContain(num) => {
                if let Some(val) = v.as_array().map(|x| {
                    x.into_iter()
                        .filter_map(|y| y.as_f64())
                        .collect::<Vec<f64>>()
                }) {
                    if !val.contains(&num) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatIn(ref nums) => {
                if let Some(val) = v.as_f64() {
                    if nums.iter().any(|&num| num == val) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatNotIn(ref nums) => {
                if let Some(val) = v.as_f64() {
                    if nums.iter().all(|&num| num != val) {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatInRange(start, end) => {
                if let Some(val) = v.as_f64() {
                    if start <= val && val <= end {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatNotInRange(start, end) => {
                if let Some(val) = v.as_f64() {
                    if start <= val && val <= end {
                        Status::NotMet
                    } else {
                        Status::Met
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatLessThan(num) => {
                if let Some(val) = v.as_f64() {
                    if val < num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatLessThanInclusive(num) => {
                if let Some(val) = v.as_f64() {
                    if val <= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatGreaterThan(num) => {
                if let Some(val) = v.as_f64() {
                    if val > num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::FloatGreaterThanInclusive(num) => {
                if let Some(val) = v.as_f64() {
                    if val >= num {
                        Status::Met
                    } else {
                        Status::NotMet
                    }
                } else {
                    Status::NotMet
                }
            }
            Constraint::BoolEquals(b) => {
                if let Some(val) = v.as_bool() {
                    if val == b {
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

// ***********************************************************************
// Rule RESULT
// **********************************************************************
/// Result of checking a rules tree.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConditionResult {
    /// Human-friendly description of the rule
    pub name: String,
    /// top-level status of this result
    pub status: Status,
    /// Results of any sub-rules
    pub children: Vec<ConditionResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuleResult {
    pub condition_result: ConditionResult,
    pub event: Event,
}

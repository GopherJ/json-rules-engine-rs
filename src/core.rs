use crate::error::{Error, Result};

use std::{
    collections::HashMap,
    ops::{BitAnd, BitOr, Not},
    time::Instant,
};

use futures_util::future::{join_all, FutureExt, TryFutureExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::to_value, Map as SerdeMap, Value};

#[cfg(feature = "eval")]
use rhai::{
    def_package,
    packages::{
        ArithmeticPackage, BasicArrayPackage, BasicMapPackage, LogicPackage,
        Package,
    },
    serde::to_dynamic,
    Engine as RhaiEngine, Map, RegisterFn, Scope,
};

#[cfg(feature = "eval")]
def_package!(rhai:JsonRulesEnginePackage:"Package for json-rules-engine", lib, {
    ArithmeticPackage::init(lib);
    LogicPackage::init(lib);
    BasicArrayPackage::init(lib);
    BasicMapPackage::init(lib);
});

#[cfg(feature = "email")]
use sendgrid::v3::{Content, Email, Message, Personalization, Sender};

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
    #[cfg(feature = "eval")]
    Eval {
        expr: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParams {
    #[serde(rename = "type")]
    pub ty: String,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "params")]
pub enum Event {
    Message(EventParams),
    PostToCallbackUrl {
        callback_url: String,
        #[serde(flatten)]
        params: EventParams,
        #[serde(default)]
        app_data: SerdeMap<String, Value>,
    },
    #[cfg(feature = "email")]
    EmailNotification {
        from: String,
        to: Vec<String>,
        #[serde(flatten)]
        params: EventParams,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CoalescenceEvent {
    coalescence: Option<u64>,
    coalescence_group: Option<String>,
    #[serde(flatten)]
    event: Event,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    pub conditions: Condition,
    pub events: Vec<CoalescenceEvent>,
}

impl Rule {
    pub fn check_value(
        &self,
        info: &Value,
        #[cfg(feature = "eval")] rhai_engine: &RhaiEngine,
    ) -> RuleResult {
        let condition_result = self.conditions.check_value(
            info,
            #[cfg(feature = "eval")]
            rhai_engine,
        );

        let mut events = self.events.to_owned();

        for CoalescenceEvent {
            event,
            coalescence_group,
            ..
        } in &mut events
        {
            let params = match event {
                Event::Message(ref mut params) => params,
                Event::PostToCallbackUrl { ref mut params, .. } => params,
                #[cfg(feature = "email")]
                Event::EmailNotification { ref mut params, .. } => params,
            };

            if let Ok(message) = mustache::compile_str(&params.message)
                .and_then(|template| template.render_to_string(info))
            {
                params.message = message;
            }

            if let Event::PostToCallbackUrl {
                ref mut callback_url,
                ..
            } = event
            {
                if let Ok(new_callback_url) =
                    mustache::compile_str(callback_url)
                        .and_then(|template| template.render_to_string(info))
                {
                    *callback_url = new_callback_url;
                }
            }

            if let Some(coalescence_group) = coalescence_group {
                if let Ok(new_coalescence_group) =
                    mustache::compile_str(coalescence_group)
                        .and_then(|template| template.render_to_string(info))
                {
                    *coalescence_group = new_coalescence_group;
                }
            }
        }

        RuleResult {
            condition_result,
            events,
        }
    }
}

#[derive(Debug)]
pub struct Engine {
    rules: Vec<Rule>,
    client: Client,
    #[cfg(feature = "email")]
    sender: Sender,
    #[cfg(feature = "eval")]
    rhai_engine: RhaiEngine,
    coalescences: HashMap<String, (Instant, u64)>,
}

impl Engine {
    pub fn new<S: Into<String>>(#[cfg(feature = "email")] api_key: S) -> Self {
        Self {
            rules: Vec::new(),
            client: Client::new(),
            #[cfg(feature = "email")]
            sender: Sender::new(api_key.into()),
            #[cfg(feature = "eval")]
            rhai_engine: {
                let mut engine = RhaiEngine::new_raw();
                engine.load_package(JsonRulesEnginePackage::new().get());
                engine
            },
            coalescences: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule)
    }

    pub fn add_rules(&mut self, rules: Vec<Rule>) {
        self.rules.extend(rules)
    }

    pub fn load_rules(&mut self, rules: Vec<Rule>) {
        self.rules = rules;
    }

    pub fn clear(&mut self) {
        self.rules.clear();
    }

    #[cfg(feature = "eval")]
    pub fn add_function(&mut self, fname: &str, f: fn(Map) -> bool) {
        self.rhai_engine.register_fn(fname, f);
    }

    pub async fn run<T: Serialize>(
        &mut self,
        facts: &T,
    ) -> Result<Vec<RuleResult>> {
        let facts = to_value(facts)?;
        let mut rule_results: Vec<RuleResult> = self
            .rules
            .iter()
            .map(|rule| {
                rule.check_value(
                    &facts,
                    #[cfg(feature = "eval")]
                    &self.rhai_engine,
                )
            })
            .filter(|rule_result| {
                rule_result.condition_result.status == Status::Met
            })
            .collect();

        self.coalescences.retain(|_k, (start, expiration)| {
            start.elapsed().as_secs() < *expiration
        });

        for rule_result in rule_results.iter_mut() {
            rule_result.events.retain(|event| {
                if let (Some(coalescence_group), Some(coalescence)) = (
                    event.coalescence_group.to_owned(),
                    event.coalescence.to_owned(),
                ) {
                    if self.coalescences.contains_key(&coalescence_group) {
                        return false;
                    } else {
                        self.coalescences.insert(
                            coalescence_group,
                            (Instant::now(), coalescence),
                        );
                    }
                }

                true
            });
        }

        let requests = rule_results
            .iter()
            .map(|rule_result| {
                rule_result
                    .events
                    .iter()
                    .map(|CoalescenceEvent { event, .. }| {
                        (event, self.client.clone(), self.sender.clone())
                    })
                    .filter_map(|(event, client, sender)| match event {
                        Event::PostToCallbackUrl {
                            ref callback_url,
                            ref params,
                            ref app_data,
                        } => Some(
                            client
                                .post(callback_url)
                                .json(&json!({
                                    "event": params,
                                    "facts": &facts,
                                    "app_data": app_data
                                }))
                                .send()
                                .map_err(Error::from)
                                .boxed(),
                        ),
                        #[cfg(feature = "email")]
                        Event::EmailNotification {
                            ref from,
                            ref to,
                            ref params,
                        } if !to.is_empty() => {
                            let p = {
                                let mut p = Personalization::new(Email::new(
                                    &to[0].to_owned(),
                                ));
                                for x in to.iter().skip(1) {
                                    p = p.add_to(Email::new(x));
                                }
                                p
                            };

                            let m = Message::new(Email::new(from))
                                .set_subject(&params.title)
                                .add_content(
                                    Content::new()
                                        .set_content_type("text/plain")
                                        .set_value(&params.message),
                                )
                                .add_personalization(p);

                            Some(
                                async move {
                                    sender.send(&m).map_err(Error::from).await
                                }
                                .boxed(),
                            )
                        }
                        _ => None,
                    })
            })
            .flatten()
            .collect::<Vec<_>>();

        let _ = join_all(requests).await;

        Ok(rule_results)
    }
}

impl Condition {
    /// Starting at this node, recursively check (depth-first) any child nodes and
    /// aggregate the results
    pub fn check_value(
        &self,
        info: &Value,
        #[cfg(feature = "eval")] rhai_engine: &RhaiEngine,
    ) -> ConditionResult {
        match *self {
            Condition::And { ref and } => {
                let mut status = Status::Met;
                let children = and
                    .iter()
                    .map(|c| {
                        c.check_value(
                            info,
                            #[cfg(feature = "eval")]
                            rhai_engine,
                        )
                    })
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
                    .map(|c| {
                        c.check_value(
                            info,
                            #[cfg(feature = "eval")]
                            rhai_engine,
                        )
                    })
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
                    .map(|c| {
                        c.check_value(
                            info,
                            #[cfg(feature = "eval")]
                            rhai_engine,
                        )
                    })
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
                let path = if field.starts_with("/") {
                    field.to_owned()
                } else {
                    format!("/{}", field)
                };

                let status = if let Some(s) = info.pointer(&path) {
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
            #[cfg(feature = "eval")]
            Condition::Eval { ref expr } => {
                let mut scope = Scope::new();
                if let Ok(val) = to_dynamic(info) {
                    scope.push_dynamic("facts", val);
                }
                let status = if rhai_engine
                    .eval_with_scope::<bool>(&mut scope, expr)
                    .unwrap_or(false)
                {
                    Status::Met
                } else {
                    Status::NotMet
                };

                ConditionResult {
                    name: "Eval".to_owned(),
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
            Constraint::StringContainsAll(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.into_iter()
                        .filter_map(|y| y.as_str())
                        .collect::<Vec<&str>>()
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
                    x.into_iter()
                        .filter_map(|y| y.as_str())
                        .collect::<Vec<&str>>()
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
            Constraint::StringDoesNotContainAny(ref s) => {
                if let Some(v) = v.as_array().map(|x| {
                    x.into_iter()
                        .filter_map(|y| y.as_str())
                        .collect::<Vec<&str>>()
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
                    x.into_iter()
                        .filter_map(|y| y.as_i64())
                        .collect::<Vec<i64>>()
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
                    x.into_iter()
                        .filter_map(|y| y.as_i64())
                        .collect::<Vec<i64>>()
                }) {
                    if nums.into_iter().all(|num| v.contains(&num)) {
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
                    x.into_iter()
                        .filter_map(|y| y.as_i64())
                        .collect::<Vec<i64>>()
                }) {
                    if nums.into_iter().any(|num| v.contains(&num)) {
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
                    x.into_iter()
                        .filter_map(|y| y.as_i64())
                        .collect::<Vec<i64>>()
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
                    x.into_iter()
                        .filter_map(|y| y.as_i64())
                        .collect::<Vec<i64>>()
                }) {
                    if nums.into_iter().all(|num| !v.contains(&num)) {
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
                    if v == num {
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
                    if v != num {
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
                    x.into_iter()
                        .filter_map(|y| y.as_f64())
                        .collect::<Vec<f64>>()
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
                    x.into_iter()
                        .filter_map(|y| y.as_f64())
                        .collect::<Vec<f64>>()
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
                    if nums.iter().any(|&num| num == v) {
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
                    if nums.iter().all(|&num| num != v) {
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
    pub events: Vec<CoalescenceEvent>,
}

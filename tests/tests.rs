#![allow(unused_imports)]
use erased_serde::Serialize as ErasedSerialize;
#[cfg(feature = "eval")]
use json_rules_engine::{from_dynamic, Map};
use json_rules_engine::{Engine, Error, EventTrait, Rule, Status};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Mutex};

#[cfg(not(feature = "async"))]
#[test]
fn basic_met() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "name",
                    "operator": "string_equals",
                    "value": "Cheng JIANG"
                },
                {
                    "field": "age",
                    "operator": "int_in_range",
                    "value": [20, 25]
                },
                {
                    "field": "action",
                    "operator": "string_equals",
                    "value": "coding in rust"
                }
            ]
        },
        "events": [
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let facts = json!({
        "name": "Cheng JIANG",
        "age": 24,
        "action": "coding in rust",
    });

    let rule_results = engine.run(&facts).unwrap();

    assert_eq!(rule_results[0].condition_result.status, Status::Met)
}

#[cfg(not(feature = "async"))]
#[test]
fn basic_not_met() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "age",
                    "operator": "int_in_range",
                    "value": [20, 25]
                },
            ]
        },
        "events": [
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let facts = json!({
        "name": "Cheng JIANG",
        "age": 18,
        "action": "coding in rust",
    });

    let rule_results = engine.run(&facts).unwrap();

    assert_eq!(rule_results.len(), 0);
}

#[cfg(not(feature = "async"))]
#[cfg(feature = "eval")]
#[test]
fn custom_function() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    fn age_greater_than20_less_than_inclusive25(p: Map) -> bool {
        let facts: Facts = from_dynamic(&p.into()).unwrap();
        facts.age > 20 && facts.age <= 25
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "name",
                    "operator": "string_equals",
                    "value": "Cheng JIANG"
                },
                {
                    "field": "age",
                    "operator": "int_in_range",
                    "value": [20, 25]
                },
                {
                    "and": [
                        {
                            "expr": "facts.age > 20 && facts.age <= 25",
                        },
                        {
                            "expr": "my_function(facts)",
                        },
                    ]
                },
                {
                    "field": "action",
                    "operator": "string_equals",
                    "value": "coding in rust"
                }
            ]
        },
        "events": [
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);
    engine
        .add_function("my_function", age_greater_than20_less_than_inclusive25);

    let facts = json!({
        "name": "Cheng JIANG",
        "age": 24,
        "action": "coding in rust",
    });

    let rule_results = engine.run(&facts).unwrap();

    assert_eq!(rule_results[0].condition_result.status, Status::Met)
}

#[cfg(feature = "callback")]
#[tokio::test]
async fn post_callback_event() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "name",
                    "operator": "string_equals",
                    "value": "Cheng JIANG"
                },
                {
                    "field": "age",
                    "operator": "int_in_range",
                    "value": [20, 25]
                },
                {
                    "field": "action",
                    "operator": "string_equals",
                    "value": "coding in rust"
                }
            ]
        },
        "events": [
            {
                "type": "post_to_callback_url",
                "params": {
                    "callback_url": "https://example.com/whatever",
                    "type": "info",
                    "title": "Another person is coding in rust",
                    "message": "Name: {{ name }}, Age: {{ age }}, Action: {{ action }}"
                }
            }
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let facts = json!({
        "name": "Cheng JIANG",
        "age": 24,
        "action": "coding in rust",
    });

    let rule_results = engine.run(&facts).await.unwrap();

    assert_eq!(rule_results[0].condition_result.status, Status::Met)
}

#[cfg(feature = "async")]
#[tokio::test]
async fn custom_event_async() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    #[derive(Debug, Clone)]
    struct CustomEvent {
        ty: String,
        res: String,
    }

    #[async_trait::async_trait]
    impl EventTrait for CustomEvent {
        fn new() -> Self {
            Self {
                ty: "custom_event".to_string(),
                res: String::new(),
            }
        }

        fn get_type(&self) -> &str {
            &self.ty
        }

        async fn validate(
            &self,
            params: &HashMap<String, Value>,
        ) -> Result<(), String> {
            if !params.contains_key("name") {
                return Err("'name' is missing.".to_string());
            }

            Ok(())
        }

        async fn trigger(
            &mut self,
            params: &HashMap<String, Value>,
            facts: &(dyn ErasedSerialize + Sync),
        ) -> Result<(), Error> {
            let mut name =
                params.get("name").unwrap().as_str().unwrap().to_string();

            let value = serde_json::from_str::<Value>(
                &serde_json::to_string(facts).unwrap(),
            )
            .unwrap();
            if let Ok(tmpl) = mustache::compile_str(&name)
                .and_then(|template| template.render_to_string(&value))
            {
                name = tmpl;
            }

            self.res = format!("name is: {}", &name);

            Ok(())
        }
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "name",
                    "operator": "string_equals",
                    "value": "Cheng JIANG"
                },
                {
                    "field": "age",
                    "operator": "int_in_range",
                    "value": [20, 25]
                },
                {
                    "field": "action",
                    "operator": "string_equals",
                    "value": "coding in rust"
                }
            ]
        },
        "events": [
            {
                "type": "custom_event",
                "params": {
                    "name": "Cheng JIANG",
                }
            }
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let custom_event =
        std::sync::Arc::new(futures_util::lock::Mutex::new(CustomEvent::new()));
    engine.add_event(custom_event.clone());

    let facts = json!({
        "name": "Cheng JIANG",
        "age": 24,
        "action": "coding in rust",
    });

    engine.run(&facts).await.unwrap();

    assert_eq!(&custom_event.lock().unwrap().res, "name is: Cheng JIANG");
}

#[cfg(not(feature = "async"))]
#[test]
fn custom_event() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    #[derive(Debug, Clone)]
    struct CustomEvent {
        ty: String,
        res: String,
    }

    impl EventTrait for CustomEvent {
        fn new() -> Self {
            Self {
                ty: "custom_event".to_string(),
                res: String::new(),
            }
        }

        fn get_type(&self) -> &str {
            &self.ty
        }

        fn validate(
            &self,
            params: &HashMap<String, Value>,
        ) -> Result<(), String> {
            if !params.contains_key("name") {
                return Err("'name' is missing.".to_string());
            }

            Ok(())
        }

        fn trigger(
            &mut self,
            params: &HashMap<String, Value>,
            facts: &(dyn ErasedSerialize + Sync),
        ) -> Result<(), Error> {
            let mut name =
                params.get("name").unwrap().as_str().unwrap().to_string();

            let value = serde_json::from_str::<Value>(
                &serde_json::to_string(facts).unwrap(),
            )
            .unwrap();
            if let Ok(tmpl) = mustache::compile_str(&name)
                .and_then(|template| template.render_to_string(&value))
            {
                name = tmpl;
            }

            self.res = format!("name is: {}", &name);

            Ok(())
        }
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "name",
                    "operator": "string_equals",
                    "value": "Cheng JIANG"
                },
                {
                    "field": "age",
                    "operator": "int_in_range",
                    "value": [20, 25]
                },
                {
                    "field": "action",
                    "operator": "string_equals",
                    "value": "coding in rust"
                }
            ]
        },
        "events": [
            {
                "type": "custom_event",
                "params": {
                    "name": "Cheng JIANG",
                }
            }
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let custom_event = std::rc::Rc::new(Mutex::new(CustomEvent::new()));
    engine.add_event(custom_event.clone());

    let facts = json!({
        "name": "Cheng JIANG",
        "age": 24,
        "action": "coding in rust",
    });

    engine.run(&facts).unwrap();

    assert_eq!(&custom_event.lock().unwrap().res, "name is: Cheng JIANG");
}

#[cfg(not(feature = "async"))]
#[test]
fn test_a_pointer() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "person/name",
                    "operator": "string_equals",
                    "value": "Cheng JIANG"
                },
                {
                    "field": "person/age",
                    "operator": "int_in_range",
                    "value": [20, 25]
                },
            ]
        },
        "events": [
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let facts = json!({
        "person": {
            "name": "Cheng JIANG",
            "age": 24,
        }
    });

    let rule_results = engine.run(&facts).unwrap();

    assert_eq!(rule_results[0].condition_result.status, Status::Met)
}

#[cfg(not(feature = "async"))]
#[cfg(feature = "path")]
#[test]
fn test_a_pointer_and_path() {
    #[derive(Deserialize, Serialize)]
    struct Facts {
        name: String,
        age: u8,
        action: String,
    }

    let rule_json = json!({
        "conditions": {
            "and": [
                {
                    "field": "people",
                    "operator": "string_contains",
                    "value": "Cheng JIANG",
                    "path": "$..name"
                }
            ]
        },
        "events": [
        ]
    });

    let rule: Rule = serde_json::from_str::<Rule>(
        &serde_json::to_string(&rule_json).unwrap(),
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let facts = json!({
        "people": [
            {
                "name": "Cheng JIANG",
                "age": 24,
            },
            {
                "name": "Omid Rad",
                "age": 23,
            },
        ]
    });

    let rule_results = engine.run(&facts).unwrap();

    assert_eq!(rule_results[0].condition_result.status, Status::Met)
}

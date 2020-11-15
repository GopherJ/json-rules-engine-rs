# json-rules-engine

## Installation

Add this package to `Cargo.toml` of your project. (Check https://crates.io/crates/json-rules-engine for right version)

```toml
[dependencies]
json-rules-engine = { version = "0.5.0", features = ["email"] }
tokio = { version = "0.3.3", features = ["macros"] }
serde_json = { version = "*" }
anyhow = { version = "*" }
```

## Features

- Built in operators
- Full support for `ALL`, `ANY`, `AtLeast` boolean operators, including recursive nesting
- Type Safe
- Load rules from json
- HTTP post to callback url
- Built in Moustache render
- Email notifications based on `SendGrid`
- Safe script

## Get started

```rust
use json_rules_engine::{Engine, Rule};
use serde_json::json;

#[tokio::main]
async main() -> anyhow::Result<()> {
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
                    "script": "facts.age > 20 && facts.age <= 25",
                },
                {
                    "field": "action",
                    "operator": "string_equals",
                    "value": "coding in rust"
                }
            ]
        },
        "event": {
            "type": "post_to_callback_url",
            "params": {
                "callback_url": "http://example.com/people/conding_in_rust",
                "type": "info",
                "title": "Another person is coding in rust",
                "message": "Name: {{ name }}, Age: {{ age }}, Action: {{ action }},"
            }
        }
    });

    let rule: Rule = serde_json::from_str::<Rule>(&serde_json::to_string(&rule_json).unwrap()).unwrap();

    let mut engine = Engine::new();
    engine.add_rule(rule);

    let facts = json!({
        "name": "Cheng JIANG",
        "age": 24,
        "action": "coding in rust",
    });

    let rule_results = engine.run(&facts).await?;

    println!("{:?}", rule_results);
}
```

## Special Thanks

- [bsundsrud](https://github.com/bsundsrud) for its basic implementation [ruuls-rs](https://github.com/bsundsrud/ruuls-rs)

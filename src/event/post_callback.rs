use crate::{event::EventTrait, Error};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PostCallback {
    ty: String,
    client: Client,
}

#[async_trait]
impl EventTrait for PostCallback {
    fn new() -> Self {
        Self {
            ty: "post_to_callback_url".to_string(),
            client: Client::new(),
        }
    }

    fn get_type(&self) -> &str {
        &self.ty
    }

    fn validate(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        if !params.contains_key("callback_url") {
            return Err("'callback_url' is missing.".to_string());
        }

        Ok(())
    }

    async fn trigger(
        &self,
        params: &HashMap<String, serde_json::Value>,
        facts: &serde_json::Value,
    ) -> Result<(), Error> {
        let mut callback_url = params
            .get("callback_url")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        if let Ok(tmpl) = mustache::compile_str(&callback_url)
            .and_then(|template| template.render_to_string(facts))
        {
            callback_url = tmpl;
        }

        self.client
            .post(callback_url)
            .json(&json!({
                "event": params,
                "facts": facts,
            }))
            .send()
            .await?;

        Ok(())
    }
}

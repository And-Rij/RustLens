//! Local Ollama HTTP client, installed model selection, and translation prompt.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{sync::Mutex, time::Duration};

pub const DEFAULT_MODEL: &str = "translategemma:12b";
pub struct OllamaService {
    pub client: Client,
    // Serialize generation to keep concurrent windows from competing for the model.
    pub gate: tokio::sync::Mutex<()>,
    model: Mutex<String>,
}

impl Default for OllamaService {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(120))
                .build()
                .expect("HTTP client"),
            gate: tokio::sync::Mutex::new(()),
            model: Mutex::new(DEFAULT_MODEL.into()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelList {
    pub models: Vec<String>,
    pub selected: String,
}
#[derive(Deserialize)]
struct Tags {
    models: Vec<Tag>,
}
#[derive(Deserialize)]
struct Tag {
    name: String,
}
#[derive(Deserialize)]
struct ChatReply {
    message: MessageReply,
    done: bool,
    done_reason: Option<String>,
}
#[derive(Deserialize)]
struct MessageReply {
    content: String,
}

pub fn validate_text(text: &str) -> Result<&str, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Введи англійський текст для перекладу.".into());
    }
    if text.chars().count() > 4000 {
        return Err(
            "Текст надто довгий. Залиш до 4000 символів або виділи менший фрагмент.".into(),
        );
    }
    Ok(text)
}

fn http_error(error: reqwest::Error) -> String {
    if error.is_connect() {
        "Ollama недоступна. Запусти Ollama та повтори переклад.".into()
    } else if error.is_timeout() {
        "Ollama не відповіла за 120 секунд. Перевір навантаження й повтори переклад.".into()
    } else {
        format!("Помилка з'єднання з Ollama: {error}")
    }
}

/// Keep the entire source in one user message using TranslateGemma's prompt format.
pub fn translation_request(model: &str, text: &str) -> serde_json::Value {
    let prompt = format!(
        concat!(
            "You are a professional English (en) to Ukrainian (uk) translator. ",
            "Your goal is to accurately convey the meaning and nuances of the original English text ",
            "while adhering to Ukrainian grammar, vocabulary, and cultural sensitivities.\n",
            "Produce only the Ukrainian translation, without any additional explanations or commentary. ",
            "Please translate the following English text into Ukrainian:\n\n\n{}"
        ),
        text
    );
    let mut request = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "keep_alive": "60s",
        "options": {
            "temperature": 0,
            "num_ctx": 4096,
            "num_predict": 1024
        }
    });
    if model.starts_with("gemma4") {
        request["think"] = serde_json::json!(false);
    }
    request
}

impl OllamaService {
    pub fn model(&self) -> Result<String, String> {
        self.model
            .lock()
            .map(|v| v.clone())
            .map_err(|_| "Не вдалося отримати модель.".into())
    }

    pub async fn models(&self) -> Result<ModelList, String> {
        let response = self
            .client
            .get("http://127.0.0.1:11434/api/tags")
            .send()
            .await
            .map_err(http_error)?;
        let response = response.error_for_status().map_err(http_error)?;
        let tags: Tags = response.json().await.map_err(http_error)?;
        Ok(ModelList {
            models: tags.models.into_iter().map(|m| m.name).collect(),
            selected: self.model()?,
        })
    }

    pub async fn select_model(&self, model: String) -> Result<(), String> {
        if !self.models().await?.models.contains(&model) {
            return Err("Модель не встановлена в Ollama. Онови список моделей.".into());
        }
        *self
            .model
            .lock()
            .map_err(|_| "Не вдалося вибрати модель.")? = model;
        Ok(())
    }

    pub async fn translate(&self, text: &str, model: &str) -> Result<String, String> {
        let text = validate_text(text)?;
        let response = self
            .client
            .post("http://127.0.0.1:11434/api/chat")
            .json(&translation_request(model, text))
            .send()
            .await
            .map_err(http_error)?;
        if !response.status().is_success() {
            let status = response.status();
            let error: serde_json::Value = response.json().await.unwrap_or_default();
            return Err(format!(
                "Ollama: {status}. {}",
                error["error"]
                    .as_str()
                    .unwrap_or("Перевір модель і повтори переклад.")
            ));
        }
        let reply: ChatReply = response.json().await.map_err(http_error)?;
        if !reply.done || reply.done_reason.as_deref() == Some("length") {
            return Err("Переклад не завершено. Спробуй коротший фрагмент.".into());
        }
        let translation = reply.message.content.trim().to_string();
        if translation.is_empty() {
            return Err("Модель повернула порожню відповідь. Спробуй повторити переклад.".into());
        }
        Ok(translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_and_overlong_inputs() {
        assert!(validate_text(" \n ").is_err());
        assert!(validate_text(&"x".repeat(4001)).is_err());
        assert_eq!(validate_text("  Keep going.\n").unwrap(), "Keep going.");
    }

    #[test]
    fn prompt_keeps_game_text_in_one_user_message() {
        let request = translation_request(DEFAULT_MODEL, "Open the gate.\nWatch out!");
        assert_eq!(request["messages"].as_array().unwrap().len(), 1);
        assert!(
            request["messages"][0]["content"]
                .as_str()
                .unwrap()
                .ends_with("\n\n\nOpen the gate.\nWatch out!")
        );
        assert_eq!(request["stream"], false);
    }
    #[tokio::test]
    #[ignore = "Requires local Ollama and TranslateGemma"]
    async fn translates_using_local_ollama() {
        let result = OllamaService::default()
            .translate(
                "We are running out of supplies. Keep an eye on the northern gate.",
                DEFAULT_MODEL,
            )
            .await
            .unwrap();
        println!("Translation: {result}");
        assert!(
            result
                .chars()
                .any(|c| ('а'..='я').contains(&c) || "іїєґ".contains(c))
        );
    }
}

use crate::llm::provider::ChatRequest;
use anyhow::Result;

pub trait Middleware: Send + Sync {
    /// Intercepts and potentially modifies a ChatRequest before it is sent to the LLM.
    fn process_request(&self, request: &mut ChatRequest) -> Result<()>;
}

/// A middleware that scrubs basic PII (Personally Identifiable Information)
/// like emails and phone numbers from user messages before sending them to the LLM.
pub struct PIIScrubber;

impl Middleware for PIIScrubber {
    fn process_request(&self, request: &mut ChatRequest) -> Result<()> {
        let email_regex = regex::Regex::new(r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}").unwrap();
        // A very basic phone number regex for demonstration purposes
        let phone_regex = regex::Regex::new(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();

        for msg in &mut request.messages {
            let original_content = msg.content.clone();

            // Scrub emails
            let mut scrubbed = email_regex
                .replace_all(&original_content, "[EMAIL REDACTED]")
                .to_string();

            // Scrub phones
            scrubbed = phone_regex
                .replace_all(&scrubbed, "[PHONE REDACTED]")
                .to_string();

            msg.content = scrubbed;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::Message;

    #[test]
    fn test_pii_scrubber() {
        let scrubber = PIIScrubber;
        let mut req = ChatRequest {
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content:
                        "My email is test@example.com and phone is 123-456-7890. Please call me!"
                            .to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    images: None,
                },
                Message {
                    role: "assistant".to_string(),
                    content: "This is a safe message with no PII.".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    images: None,
                },
            ],
            tools: None,
        };

        scrubber.process_request(&mut req).unwrap();

        assert_eq!(
            req.messages[0].content,
            "My email is [EMAIL REDACTED] and phone is [PHONE REDACTED]. Please call me!"
        );
        assert_eq!(
            req.messages[1].content,
            "This is a safe message with no PII."
        );
    }
}

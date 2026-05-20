use anyhow::{anyhow, Result};
use io_jmap::client::JmapClientStd;

pub struct JmapMessageGetHandler {
    pub id: String,
}

impl JmapMessageGetHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<String> {
        let output = client.email_get(vec![self.id], None, true, true, 0)?;

        let email = output
            .emails
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Email not found"))?;

        if let (Some(text_body), Some(body_values)) = (&email.text_body, &email.body_values) {
            if let Some(part) = text_body.first() {
                if let Some(part_id) = &part.part_id {
                    if let Some(bv) = body_values.get(part_id) {
                        return Ok(bv.value.clone());
                    }
                }
            }
        }

        if let (Some(html_body), Some(body_values)) = (&email.html_body, &email.body_values) {
            if let Some(part) = html_body.first() {
                if let Some(part_id) = &part.part_id {
                    if let Some(bv) = body_values.get(part_id) {
                        return Ok(html2text::from_read(bv.value.as_bytes(), 80));
                    }
                }
            }
        }

        Ok(email.preview.clone().unwrap_or_default())
    }
}

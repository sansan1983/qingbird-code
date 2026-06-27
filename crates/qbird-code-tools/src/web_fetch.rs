use async_trait::async_trait;

use crate::registry::{Tool, ToolDefinition, ToolOutput};
use qbird_code_models::{EflowError, Result, RiskLevel};
use rust_i18n::t;

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024; // 2 MB
const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".into(),
            description: t!("tool_web_fetch_description").to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "要获取的 URL"},
                    "format": {
                        "type": "string",
                        "enum": ["text", "markdown", "html"],
                        "description": "输出格式，默认 markdown"
                    }
                },
                "required": ["url"]
            }),
            risk_level: RiskLevel::L0,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput> {
        let url = params["url"].as_str().ok_or_else(|| {
            EflowError::Tool(t!("err_tool_missing_param", name = "url").to_string())
        })?;

        let format = params["format"].as_str().unwrap_or("markdown");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .user_agent("qingbird/0.2.0")
            .build()
            .map_err(|e| {
                EflowError::Tool(t!("err_tool_http_client", msg = e.to_string()).to_string())
            })?;

        let response = client.get(url).send().await.map_err(|e| {
            EflowError::Tool(
                t!("err_tool_http_request", url = url, msg = e.to_string()).to_string(),
            )
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(EflowError::Tool(
                t!("err_tool_http_status", url = url, code = status.as_u16()).to_string(),
            ));
        }

        let bytes = response.bytes().await.map_err(|e| {
            EflowError::Tool(
                t!("err_tool_read_response", url = url, msg = e.to_string()).to_string(),
            )
        })?;

        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(EflowError::Tool(
                t!(
                    "err_tool_http_too_large",
                    url = url,
                    size = bytes.len(),
                    limit = MAX_RESPONSE_BYTES
                )
                .to_string(),
            ));
        }

        let content = match format {
            "html" => String::from_utf8_lossy(&bytes).to_string(),
            "text" | "markdown" => {
                let body = String::from_utf8_lossy(&bytes);
                if format == "text" {
                    strip_html(&body)
                } else {
                    html_to_markdown(&body)
                }
            }
            _ => String::from_utf8_lossy(&bytes).to_string(),
        };

        let size = bytes.len();
        Ok(ToolOutput {
            success: true,
            content: t!(
                "tool_web_fetch_result",
                url = url,
                size = size,
                format = format
            )
            .to_string()
                + "\n"
                + &content,
            metadata: Some(serde_json::json!({
                "url": url,
                "format": format,
                "size": size,
                "status": status.as_u16(),
            })),
        })
    }
}

fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity_buf = String::new();

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            '&' if !in_tag => {
                in_entity = true;
                entity_buf.clear();
            }
            ';' if in_entity => {
                in_entity = false;
                let decoded = match entity_buf.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "nbsp" => " ",
                    "apos" => "'",
                    _ => "",
                };
                result.push_str(decoded);
            }
            _ if in_tag => {}
            _ if in_entity => entity_buf.push(c),
            _ => result.push(c),
        }
    }
    result
}

fn html_to_markdown(html: &str) -> String {
    let mut result = String::with_capacity(html.len());

    // Strip <style> and <script> blocks
    let cleaned = strip_style_script(html);

    let mut in_tag = false;
    let mut in_attribute = false;
    let mut attr_quote = None;
    let mut in_entity = false;
    let mut entity_buf = String::new();
    let mut tag_name = String::new();
    let mut is_closing = false;

    for c in cleaned.chars() {
        match c {
            '<' if !in_tag => {
                in_tag = true;
                in_attribute = false;
                attr_quote = None;
                tag_name.clear();
                is_closing = false;
            }
            '>' if in_tag && !in_attribute => {
                in_tag = false;
                match tag_name.as_str() {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        result.push('\n');
                        let level = tag_name[1..].parse::<usize>().unwrap_or(1);
                        for _ in 0..level {
                            result.push('#');
                        }
                        result.push(' ');
                    }
                    "br" | "hr" => result.push('\n'),
                    "p" | "div" | "section" | "article" | "li" if !is_closing => {
                        result.push('\n');
                    }
                    "tr" if !is_closing => result.push('\n'),
                    "td" | "th" if !is_closing => result.push('|'),
                    _ => {}
                }
            }
            '"' | '\'' if in_tag && !tag_name.is_empty() => {
                if attr_quote == Some(c) {
                    in_attribute = false;
                    attr_quote = None;
                } else if attr_quote.is_none() {
                    in_attribute = true;
                    attr_quote = Some(c);
                }
            }
            '/' if in_tag && tag_name.is_empty() && !in_attribute => {
                is_closing = true;
            }
            ' ' | '\t' | '\n' if in_tag && !tag_name.is_empty() && !in_attribute => {
                // start of attribute; stay in_tag
            }
            _ if in_tag => {
                if !in_attribute && (c.is_ascii_alphabetic() || c == '-') {
                    tag_name.push(c.to_ascii_lowercase());
                }
            }
            '&' if !in_tag => {
                in_entity = true;
                entity_buf.clear();
            }
            ';' if in_entity => {
                in_entity = false;
                let decoded = match entity_buf.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "nbsp" => " ",
                    "apos" => "'",
                    _ => "",
                };
                result.push_str(decoded);
            }
            _ if in_entity => entity_buf.push(c),
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    // Collapse multiple newlines
    let mut out = String::with_capacity(result.len());
    let mut prev_newline = false;
    for c in result.chars() {
        if c == '\n' {
            if !prev_newline {
                out.push('\n');
            }
            prev_newline = true;
        } else {
            out.push(c);
            prev_newline = false;
        }
    }

    out
}

fn strip_style_script(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Check for <style, <script, <!--
            let mut buf = String::new();
            while let Some(&n) = chars.peek() {
                if n == '>' || n.is_ascii_whitespace() {
                    break;
                }
                buf.push(n);
                chars.next();
            }
            let tag = buf.to_ascii_lowercase();
            if tag == "style" || tag == "script" {
                while let Some(n) = chars.next() {
                    if n == '<' {
                        let mut closing = String::new();
                        while let Some(&nn) = chars.peek() {
                            if nn == '>' {
                                chars.next();
                                break;
                            }
                            closing.push(nn);
                            chars.next();
                        }
                        if closing.to_ascii_lowercase() == format!("/{}", tag) {
                            break;
                        }
                    }
                }
                continue;
            }
            if tag.starts_with("!--") {
                while let Some(n) = chars.next() {
                    if n == '-' && chars.peek() == Some(&'-') {
                        chars.next();
                        if chars.peek() == Some(&'>') {
                            chars.next();
                            break;
                        }
                    }
                }
                continue;
            }
            // Not a special tag; push the < back via buf
            result.push('<');
            for ch in buf.chars() {
                result.push(ch);
            }
        } else {
            result.push(c);
        }
    }

    result
}

// src/cli/config.rs — CLI 文本交互式配置（6 步流程）

use crate::cli::prompt::{MenuItem, prompt_input, prompt_password, select_menu};
use crate::infrastructure::llm::types::ProtocolKind;
use crate::interaction::wizard::builtin::provider::{PRESETS, PresetProvider};

/// 运行 CLI 文本配置流程。返回退出码（0=成功, 1=取消/错误）。
pub fn run() -> i32 {
    // Step 1: 欢迎
    println!("\n  ╔══ LLM 配置向导 ══╗\n");
    println!("  未检测到 LLM 配置，开始设置。\n");
    println!("  按 Enter 继续...");
    let mut buf = String::new();
    if std::io::stdin().read_line(&mut buf).is_err() {
        return 1;
    }

    // Step 2: 选厂商
    let all_items = build_provider_menu();
    let sel = match select_menu(&all_items) {
        Some(s) => s,
        None => return 0, // 取消视为正常退出
    };

    let is_custom = sel >= PRESETS.len();
    let preset: Option<&PresetProvider> = if !is_custom {
        Some(&PRESETS[sel])
    } else {
        None
    };

    // 自定义路径：先问 protocol 和 base_url
    let (protocol_kind, base_url) = if is_custom {
        let proto_items = vec![
            MenuItem {
                key: "1",
                label: "OpenAI 兼容".into(),
            },
            MenuItem {
                key: "2",
                label: "Anthropic 兼容".into(),
            },
        ];
        println!("\n  选择协议类型:");
        let proto_sel = match select_menu(&proto_items) {
            Some(s) => s,
            None => return 0,
        };
        let kind = if proto_sel == 0 {
            ProtocolKind::OpenaiCompatible
        } else {
            ProtocolKind::AnthropicCompatible
        };
        let url = prompt_input("Base URL:");
        (kind, url)
    } else if let Some(p) = preset {
        (p.protocol, p.base_url.to_string())
    } else {
        return 1;
    };

    // Step 3: 填 Key
    let api_key = prompt_password("API Key:");

    // Step 4: 拉模型列表
    let mut models: Vec<String> = Vec::new();
    if let Some(p) = preset {
        models = p.preset_models.iter().map(|s| s.to_string()).collect();
    }
    if !api_key.is_empty() {
        println!("  正在拉取可用模型...");
        let url = format!("{}/models", base_url.trim_end_matches('/'));
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            && let Ok(resp) = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
            && let Ok(json) = resp.json::<serde_json::Value>()
            && let Some(data) = json.get("data").and_then(|d| d.as_array())
        {
            for m in data {
                if let Some(id) = m.get("id").and_then(|id| id.as_str()) {
                    let id = id.to_string();
                    if !models.contains(&id) {
                        models.push(id);
                    }
                }
            }
        }
    }

    // Step 5: 选模型
    let model = if !models.is_empty() {
        let model_items: Vec<MenuItem> = models
            .iter()
            .map(|m| MenuItem {
                key: "",
                label: m.clone(),
            })
            .collect();
        match select_menu(&model_items) {
            Some(i) => models[i].clone(),
            None => return 0,
        }
    } else {
        prompt_input("手填模型 ID:")
    };

    // Step 6: 写入文件
    let provider_id = if is_custom {
        "custom"
    } else if let Some(p) = preset {
        p.id
    } else {
        return 1;
    };
    let display_name = if is_custom {
        "Custom"
    } else if let Some(p) = preset {
        p.display_name
    } else {
        return 1;
    };
    let proto_str = match protocol_kind {
        ProtocolKind::OpenaiCompatible => "openai",
        ProtocolKind::AnthropicCompatible => "anthropic",
    };

    write_provider(
        provider_id,
        display_name,
        proto_str,
        &base_url,
        &api_key,
        &model,
    );

    println!("\n  ✓ 配置已保存");
    println!("  重新运行 eflow 开始使用。\n");
    0
}

pub fn check_llm_configured() -> bool {
    let provider_dir = dirs::config_dir()
        .map(|p| p.join("eflow").join("providers"))
        .unwrap_or_else(|| std::path::PathBuf::from("./providers"));
    if let Ok(entries) = std::fs::read_dir(&provider_dir) {
        for entry in entries.flatten() {
            if entry
                .path()
                .extension()
                .map(|e| e == "yaml")
                .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

fn build_provider_menu() -> Vec<MenuItem> {
    let mut items: Vec<MenuItem> = PRESETS
        .iter()
        .map(|p| MenuItem {
            key: "",
            label: p.display_name.to_string(),
        })
        .collect();
    items.push(MenuItem {
        key: "",
        label: "自定义（兼容 OpenAI / Anthropic）".into(),
    });
    items
}

fn write_provider(
    id: &str,
    name: &str,
    protocol: &str,
    base_url: &str,
    _api_key: &str,
    model: &str,
) {
    use std::io::Write;

    let home = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("eflow")
        .join("providers");
    let _ = std::fs::create_dir_all(&home);

    let path = home.join(format!("{}.yaml", id));
    let env_var = format!("{}_API_KEY", id.to_uppercase().replace('-', "_"));
    let yaml = format!(
        r#"id: {}
display_name: "{}"
protocol: {}_compatible
base_url: "{}"
api_key: "${{{}}}"
default_model: "{}"
"#,
        id, name, protocol, base_url, env_var, model
    );
    if let Ok(mut file) = std::fs::File::create(&path) {
        let _ = file.write_all(yaml.as_bytes());
    }

    // 更新 eflow.yaml routing
    update_eflow_yaml(id);
}

fn update_eflow_yaml(provider_id: &str) {
    let path = std::path::Path::new("eflow.yaml");
    if !path.exists() {
        return;
    }
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut output = String::new();
    let mut in_routing = false;
    for line in content.lines() {
        if line.trim() == "routing:" {
            in_routing = true;
            output.push_str(line);
            output.push('\n');
            output.push_str(&format!("    strong: {}\n", provider_id));
            output.push_str(&format!("    medium: {}\n", provider_id));
            output.push_str(&format!("    light: {}\n", provider_id));
        } else if in_routing {
            // 跳过旧的 routing 子项（以空格开头的行），直到非缩进行
            if !line.starts_with(' ') && !line.starts_with("  ") {
                in_routing = false;
                output.push_str(line);
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    // 更新 eflow.yaml
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, output);
}

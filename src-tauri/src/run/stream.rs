//! Interpretação das linhas `stream-json` do `claude -p`. Funções puras: devolvem
//! eventos neutros ([`AgentEvent`]) e não sabem de banco nem de processo.

use serde_json::Value;

use super::runtime::{AgentEvent, CycleUsage, Finish};

/// Negações detalhadas por resultado; o resto só entra na contagem.
const MAX_DENIALS: usize = 5;

pub fn parse(line: &str) -> Vec<AgentEvent> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return if line.trim().is_empty() {
            Vec::new()
        } else {
            vec![AgentEvent::Log(line.to_owned())]
        };
    };
    let mut events = Vec::new();
    match value["type"].as_str() {
        Some("system") => {
            if let Some(session_id) = value["session_id"].as_str() {
                events.push(AgentEvent::Started {
                    session_id: session_id.to_owned(),
                });
            }
        }
        Some("assistant") => {
            for block in value["message"]["content"].as_array().into_iter().flatten() {
                match block["type"].as_str() {
                    Some("text") => {
                        if let Some(text) = block["text"].as_str() {
                            events.push(AgentEvent::Text(text.to_owned()));
                        }
                    }
                    Some("tool_use") => events.push(AgentEvent::Tool(describe_tool(block))),
                    _ => {}
                }
            }
        }
        Some("result") => {
            let denials = value["permission_denials"].as_array();
            events.push(AgentEvent::Finished(Finish {
                ok: value["is_error"] == false && value["subtype"] == "success",
                subtype: value["subtype"].as_str().unwrap_or("?").to_owned(),
                summary: value["result"].as_str().unwrap_or_default().to_owned(),
                session_id: value["session_id"].as_str().map(str::to_owned),
                usage: usage(&value),
                denials: denials.map_or(0, Vec::len),
                structured: value
                    .get("structured_output")
                    .filter(|output| !output.is_null())
                    .map(Value::to_string),
            }));
            for denial in denials.into_iter().flatten().take(MAX_DENIALS) {
                events.push(AgentEvent::Denied(describe(
                    &denial["tool_name"],
                    &denial["tool_input"],
                )));
            }
        }
        _ => {}
    }
    events
}

/// Uso do ciclo. Campo ausente vira `None`, nunca zero.
fn usage(result: &Value) -> CycleUsage {
    let tokens = &result["usage"];
    CycleUsage {
        input: tokens["input_tokens"].as_u64(),
        output: tokens["output_tokens"].as_u64(),
        cache_read: tokens["cache_read_input_tokens"].as_u64(),
        cache_write: tokens["cache_creation_input_tokens"].as_u64(),
        cost_usd: result["total_cost_usd"].as_f64(),
        turns: result["num_turns"].as_u64(),
    }
}

fn describe_tool(block: &Value) -> String {
    describe(&block["name"], &block["input"])
}

fn describe(name: &Value, input: &Value) -> String {
    let name = name.as_str().unwrap_or("?");
    let detail = input["command"]
        .as_str()
        .or_else(|| input["file_path"].as_str())
        .or_else(|| input["pattern"].as_str())
        .unwrap_or("");
    if detail.is_empty() {
        name.to_owned()
    } else {
        format!("{name}: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(events: &[AgentEvent]) -> &Finish {
        events
            .iter()
            .find_map(|event| match event {
                AgentEvent::Finished(finish) => Some(finish),
                _ => None,
            })
            .expect("evento de fim")
    }

    #[test]
    fn extracts_text_tools_and_result() {
        let assistant = r#"{"type":"assistant","message":{"content":[
            {"type":"text","text":"Vou ler o arquivo."},
            {"type":"tool_use","name":"Bash","input":{"command":"bun run check:quick"}},
            {"type":"tool_use","name":"Read","input":{"file_path":"src/a.ts"}}]}}"#
            .replace('\n', "");
        assert_eq!(
            parse(&assistant),
            vec![
                AgentEvent::Text("Vou ler o arquivo.".into()),
                AgentEvent::Tool("Bash: bun run check:quick".into()),
                AgentEvent::Tool("Read: src/a.ts".into()),
            ]
        );

        let result = r#"{"type":"result","subtype":"success","is_error":false,"result":"Pronto.","num_turns":4,"total_cost_usd":0.1234,"permission_denials":[{}]}"#;
        let events = parse(result);
        let finish = finished(&events);
        assert_eq!(
            finish.line(),
            "success · 4 turnos · US$ 0.12 · 1 ações negadas"
        );
        assert!(finish.ok);
        assert_eq!(finish.summary, "Pronto.");
        assert_eq!(finish.usage.turns, Some(4));
        assert_eq!(finish.usage.cost_usd, Some(0.1234));
    }

    #[test]
    fn usage_is_read_per_field_and_missing_means_unknown() {
        let full = r#"{"type":"result","subtype":"success","is_error":false,"result":"ok",
            "session_id":"s-1","num_turns":2,"total_cost_usd":0.5,
            "usage":{"input_tokens":100,"output_tokens":20,"cache_read_input_tokens":80,"cache_creation_input_tokens":5}}"#
            .replace('\n', "");
        let events = parse(&full);
        let finish = finished(&events);
        assert_eq!(finish.session_id.as_deref(), Some("s-1"));
        assert_eq!(finish.usage.input, Some(100));
        assert_eq!(finish.usage.output, Some(20));
        assert_eq!(finish.usage.cache_read, Some(80));
        assert_eq!(finish.usage.cache_write, Some(5));
        assert_eq!(finish.usage.cost_usd, Some(0.5));
        assert_eq!(finish.usage.turns, Some(2));
        assert_eq!(finish.line(), "success · 2 turnos · US$ 0.50");

        let bare = r#"{"type":"result","subtype":"success","is_error":false,"result":"ok"}"#;
        let events = parse(bare);
        let finish = finished(&events);
        assert_eq!(finish.usage, CycleUsage::default());
        assert_eq!(finish.session_id, None);
        assert_eq!(finish.structured, None);

        let structured = r#"{"type":"result","subtype":"success","is_error":false,"result":"ok","structured_output":{"veredito":"approve"}}"#;
        let events = parse(structured);
        assert_eq!(
            finished(&events).structured.as_deref(),
            Some(r#"{"veredito":"approve"}"#)
        );
        assert_eq!(
            finish.line(),
            "success · turnos não informados · custo não informado"
        );
    }

    #[test]
    fn session_id_comes_from_the_system_line() {
        let init = r#"{"type":"system","subtype":"init","session_id":"abc-123","model":"x"}"#;
        assert_eq!(
            parse(init),
            vec![AgentEvent::Started {
                session_id: "abc-123".into()
            }]
        );
        assert!(parse(r#"{"type":"system","subtype":"init"}"#).is_empty());
    }

    #[test]
    fn denied_actions_become_events_after_the_result() {
        let result = r#"{"type":"result","subtype":"success","is_error":false,"result":"ok","permission_denials":[
            {"tool_name":"Bash","tool_use_id":"a","tool_input":{"command":"git push origin main"}},
            {"tool_name":"WebFetch","tool_input":{"url":"https://x"}},
            {"tool_use_id":"c"}]}"#
            .replace('\n', "");
        let events = parse(&result);
        assert!(matches!(events[0], AgentEvent::Finished(_)));
        let kinds: Vec<_> = events
            .into_iter()
            .filter_map(|event| match event {
                AgentEvent::Denied(text) => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, ["Bash: git push origin main", "WebFetch", "?"]);

        let none = r#"{"type":"result","subtype":"success","is_error":false,"result":"ok"}"#;
        assert!(!parse(none)
            .iter()
            .any(|event| matches!(event, AgentEvent::Denied(_))));
    }

    #[test]
    fn error_results_and_noise() {
        let result = r#"{"type":"result","subtype":"error_max_turns","is_error":true}"#;
        assert!(!finished(&parse(result)).ok);
        assert!(parse(r#"{"type":"user"}"#).is_empty());
        assert_eq!(
            parse("texto solto"),
            vec![AgentEvent::Log("texto solto".to_owned())]
        );
        assert!(parse("").is_empty());
    }
}

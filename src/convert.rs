//! Conversão entre json, yaml e toml, passando por `serde_json::Value`.

use anyhow::{Context, Result, anyhow};
use serde_json::Value;

use crate::detect::Kind;

/// Formatos estruturados que sabemos ler e escrever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
    Toml,
}

impl Format {
    /// `None` se `kind` não é um formato estruturado.
    pub fn from_kind(kind: Kind) -> Option<Self> {
        match kind {
            Kind::Json => Some(Self::Json),
            Kind::Yaml => Some(Self::Yaml),
            Kind::Toml => Some(Self::Toml),
            _ => None,
        }
    }

    pub fn kind(self) -> Kind {
        match self {
            Self::Json => Kind::Json,
            Self::Yaml => Kind::Yaml,
            Self::Toml => Kind::Toml,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
        }
    }
}

/// Adivinha o formato de um texto que a extensão não identificou, pelo
/// primeiro parser que o aceita. YAML vai por último e só vale se o resultado
/// for um mapa ou lista, porque quase todo texto é um escalar YAML válido.
pub fn sniff(text: &str) -> Option<Format> {
    [Format::Json, Format::Toml, Format::Yaml]
        .into_iter()
        .find(|&format| {
            parse(text, format)
                .is_ok_and(|v| format != Format::Yaml || v.is_object() || v.is_array())
        })
}

/// Lê `text` como `from` e o reescreve como `to`, preservando a ordem das chaves.
pub fn convert(text: &str, from: Format, to: Format) -> Result<String> {
    let value = parse(text, from)
        .with_context(|| crate::i18n::t_args("error-input-invalid", &[("format", from.name())]))?;
    serialize(&value, to).with_context(|| {
        crate::i18n::t_args(
            "error-convert-failed",
            &[("from", from.name()), ("to", to.name())],
        )
    })
}

fn parse(text: &str, format: Format) -> Result<Value> {
    Ok(match format {
        Format::Json => serde_json::from_str(text)?,
        Format::Yaml => serde_saphyr::from_str(text)?,
        Format::Toml => toml_to_json(toml::from_str(text)?),
    })
}

/// Datas e horas do TOML viram strings com o texto original. Não passamos
/// por `serde_json::to_value`, porque o `Serialize` de `toml::Datetime` emite
/// um mapa de uso interno do crate (`$__toml_private_datetime`).
fn toml_to_json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::from(i),
        // NaN e infinito não existem em JSON; viram null, como o serde_json faz.
        toml::Value::Float(f) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(items) => Value::Array(items.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, toml_to_json(value)))
                .collect(),
        ),
    }
}

fn serialize(value: &Value, format: Format) -> Result<String> {
    Ok(match format {
        Format::Json => serde_json::to_string_pretty(value)? + "\n",
        Format::Yaml => serde_saphyr::to_string(value).map_err(|e| anyhow!("{e}"))?,
        Format::Toml => toml::to_string_pretty(value)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = include_str!("../tests/examples/data.json");
    const NO_NULL: &str = include_str!("../tests/examples/data-no-null.json");
    const CONFIG_YAML: &str = include_str!("../tests/examples/config.yaml");
    const CONFIG_TOML: &str = include_str!("../tests/examples/config.toml");

    fn value(text: &str, format: Format) -> Value {
        parse(text, format).unwrap()
    }

    #[test]
    fn yaml_serializer_keeps_key_order() {
        let text = r#"{"z":1,"a":{"k":2,"b":3},"m":[{"q":1,"c":2}]}"#;
        let yaml = convert(text, Format::Json, Format::Yaml).unwrap();
        let keys: Vec<&str> = yaml
            .lines()
            .filter_map(|l| l.trim_start_matches([' ', '-']).split(':').next())
            .filter(|k| !k.is_empty())
            .collect();
        assert_eq!(keys, ["z", "a", "k", "b", "m", "q", "c"], "{yaml}");
    }

    #[test]
    fn json_yaml_json_round_trip_keeps_values_and_order() {
        for text in [DATA, NO_NULL] {
            let yaml = convert(text, Format::Json, Format::Yaml).unwrap();
            let back = convert(&yaml, Format::Yaml, Format::Json).unwrap();
            let (a, b) = (value(text, Format::Json), value(&back, Format::Json));
            assert_eq!(a, b);
            assert_eq!(a.to_string(), b.to_string(), "ordem das chaves");
        }
    }

    #[test]
    fn json_toml_json_round_trip_keeps_values_and_order() {
        let toml = convert(NO_NULL, Format::Json, Format::Toml).unwrap();
        let back = convert(&toml, Format::Toml, Format::Json).unwrap();
        let (a, b) = (value(NO_NULL, Format::Json), value(&back, Format::Json));
        assert_eq!(a.to_string(), b.to_string());
    }

    #[test]
    fn yaml_and_toml_examples_convert_to_each_other() {
        for (text, from, to) in [
            (CONFIG_YAML, Format::Yaml, Format::Toml),
            (CONFIG_TOML, Format::Toml, Format::Yaml),
        ] {
            let out = convert(text, from, to).unwrap();
            assert_eq!(value(&out, to), value(text, from));
        }
    }

    #[test]
    fn sniff_finds_the_format_and_rejects_plain_text() {
        assert_eq!(sniff(DATA), Some(Format::Json));
        assert_eq!(sniff(CONFIG_TOML), Some(Format::Toml));
        assert_eq!(sniff(CONFIG_YAML), Some(Format::Yaml));
        assert_eq!(sniff("só um texto qualquer"), None);
    }

    #[test]
    fn null_to_toml_is_a_clear_error() {
        let err = convert(DATA, Format::Json, Format::Toml).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("json") && msg.contains("toml"), "{msg}");
    }

    #[test]
    fn toml_datetimes_become_plain_strings_in_json_and_yaml() {
        let text = concat!(
            "offset = 2026-10-15T10:00:00Z\n",
            "local_date = 1979-05-27\n",
            "local_time = 07:32:00\n",
            "list = [1979-05-27, 07:32:00]\n",
            "[nested.deeper]\n",
            "when = 1979-05-27T07:32:00-03:00\n",
            "[[events]]\n",
            "at = 2026-10-15T10:00:00Z\n",
        );
        let expected = serde_json::json!({
            "offset": "2026-10-15T10:00:00Z",
            "local_date": "1979-05-27",
            "local_time": "07:32:00",
            "list": ["1979-05-27", "07:32:00"],
            "nested": {"deeper": {"when": "1979-05-27T07:32:00-03:00"}},
            "events": [{"at": "2026-10-15T10:00:00Z"}],
        });
        let json = convert(text, Format::Toml, Format::Json).unwrap();
        assert_eq!(value(&json, Format::Json), expected);
        assert!(!json.contains("$__toml_private"), "{json}");

        let yaml = convert(text, Format::Toml, Format::Yaml).unwrap();
        assert_eq!(value(&yaml, Format::Yaml), expected);
        assert!(!yaml.contains("$__toml_private"), "{yaml}");
    }

    #[test]
    fn invalid_input_is_a_clear_error() {
        let err = convert("{oops", Format::Json, Format::Yaml).unwrap_err();
        assert!(format!("{err:#}").contains("não é json válido"));
    }
}

use std::fs;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use crate::builtin::Callable;
use crate::builtin::Param;
use crate::builtin::Type;
use crate::builtin::Value;
use crate::Builtins;

#[derive(Debug, Deserialize)]
struct CustomBuiltinsJson {
    #[serde(default)]
    global: Vec<ValueJson>,
    #[serde(default, rename = "type")]
    types: Vec<TypeJson>,
}

impl From<CustomBuiltinsJson> for Builtins {
    fn from(value: CustomBuiltinsJson) -> Self {
        Self {
            global: value.global.into_iter().map(Value::from).collect(),
            r#type: value.types.into_iter().map(Type::from).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TypeJson {
    name: String,
    #[serde(default)]
    field: Vec<ValueJson>,
    #[serde(default)]
    doc: String,
}

impl From<TypeJson> for Type {
    fn from(value: TypeJson) -> Self {
        Type {
            name: value.name,
            field: value.field.into_iter().map(Value::from).collect(),
            doc: value.doc,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ValueJson {
    name: String,
    #[serde(default, rename = "type")]
    type_: String,
    #[serde(default)]
    callable: Option<CallableJson>,
    #[serde(default)]
    doc: String,
}

impl From<ValueJson> for Value {
    fn from(value: ValueJson) -> Self {
        Value {
            name: value.name,
            r#type: value.type_,
            callable: value.callable.map(Callable::from),
            doc: value.doc,
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
struct CallableJson {
    #[serde(default)]
    params: Vec<ParamJson>,
    #[serde(default = "unknown_type")]
    return_type: String,
}

impl From<CallableJson> for Callable {
    fn from(value: CallableJson) -> Self {
        Callable {
            param: value.params.into_iter().map(Param::from).collect(),
            return_type: value.return_type,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ParamJson {
    name: String,
    #[serde(default = "unknown_type", rename = "type")]
    type_: String,
    #[serde(default)]
    doc: String,
    #[serde(default)]
    default_value: String,
    #[serde(default)]
    is_mandatory: bool,
    #[serde(default)]
    is_star_arg: bool,
    #[serde(default)]
    is_star_star_arg: bool,
}

impl From<ParamJson> for Param {
    fn from(value: ParamJson) -> Self {
        Param {
            name: value.name,
            r#type: value.type_,
            doc: value.doc,
            default_value: value.default_value,
            is_mandatory: value.is_mandatory,
            is_star_arg: value.is_star_arg,
            is_star_star_arg: value.is_star_star_arg,
        }
    }
}

fn unknown_type() -> String {
    "Unknown".to_string()
}

pub fn load_custom_builtins(path: impl AsRef<Path>) -> anyhow::Result<Builtins> {
    let path = path.as_ref();
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read custom builtins from {}", path.display()))?;
    decode_custom_builtins_json(&data)
        .with_context(|| format!("failed to parse custom builtins from {}", path.display()))
}

pub fn decode_custom_builtins_json(data: &str) -> anyhow::Result<Builtins> {
    Ok(serde_json::from_str::<CustomBuiltinsJson>(data)?.into())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::decode_custom_builtins_json;
    use super::load_custom_builtins;

    #[test]
    fn decodes_custom_globals_functions_and_types() {
        let builtins = decode_custom_builtins_json(
            r#"
            {
              "global": [
                {
                  "name": "make_document",
                  "doc": "Creates a mutable document builder.",
                  "callable": {
                    "params": [],
                    "return_type": "example.DocumentBuilder"
                  }
                },
                {
                  "name": "runtime",
                  "type": "example.runtime"
                }
              ],
              "type": [
                {
                  "name": "example.runtime",
                  "doc": "Runtime namespace.",
                  "field": [
                    {
                      "name": "decode",
                      "doc": "Decodes a serialized payload.",
                      "callable": {
                        "params": [
                          {
                            "name": "payload",
                            "type": "string",
                            "is_mandatory": true
                          }
                        ],
                        "return_type": "Unknown"
                      }
                    }
                  ]
                },
                {
                  "name": "example.Request",
                  "field": [
                    {
                      "name": "metadata",
                      "type": "example.Metadata"
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap();

        assert_eq!(builtins.global.len(), 2);

        let make_document = &builtins.global[0];
        assert_eq!(make_document.name, "make_document");
        assert_eq!(make_document.doc, "Creates a mutable document builder.");
        let callable = make_document.callable.as_ref().unwrap();
        assert!(callable.param.is_empty());
        assert_eq!(callable.return_type, "example.DocumentBuilder");

        let runtime = &builtins.global[1];
        assert_eq!(runtime.name, "runtime");
        assert_eq!(runtime.r#type, "example.runtime");

        assert_eq!(builtins.r#type.len(), 2);
        let runtime_type = &builtins.r#type[0];
        assert_eq!(runtime_type.name, "example.runtime");
        assert_eq!(runtime_type.doc, "Runtime namespace.");
        assert_eq!(runtime_type.field.len(), 1);

        let decode = &runtime_type.field[0];
        assert_eq!(decode.name, "decode");
        assert_eq!(decode.doc, "Decodes a serialized payload.");
        let callable = decode.callable.as_ref().unwrap();
        assert_eq!(callable.return_type, "Unknown");
        assert_eq!(callable.param.len(), 1);
        assert_eq!(callable.param[0].name, "payload");
        assert_eq!(callable.param[0].r#type, "string");
        assert!(callable.param[0].is_mandatory);

        let request_type = &builtins.r#type[1];
        assert_eq!(request_type.name, "example.Request");
        assert_eq!(request_type.field[0].name, "metadata");
        assert_eq!(request_type.field[0].r#type, "example.Metadata");
    }

    #[test]
    fn fills_optional_defaults() {
        let builtins = decode_custom_builtins_json(
            r#"
            {
              "global": [
                {
                  "name": "make_document",
                  "callable": {}
                }
              ],
              "type": [
                {
                  "name": "example.DocumentBuilder",
                  "field": [
                    {
                      "name": "set_name",
                      "callable": {
                        "params": [
                          {
                            "name": "name"
                          }
                        ]
                      }
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap();

        let callable = builtins.global[0].callable.as_ref().unwrap();
        assert!(callable.param.is_empty());
        assert_eq!(callable.return_type, "Unknown");
        assert_eq!(builtins.global[0].doc, "");

        let method = &builtins.r#type[0].field[0];
        assert_eq!(method.doc, "");
        let param = &method.callable.as_ref().unwrap().param[0];
        assert_eq!(param.name, "name");
        assert_eq!(param.r#type, "Unknown");
        assert_eq!(param.doc, "");
        assert_eq!(param.default_value, "");
        assert!(!param.is_mandatory);
    }

    #[test]
    fn loads_custom_builtins_from_disk() {
        let path = std::env::temp_dir().join(format!(
            "starpls-custom-builtins-{}.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"
            {
              "global": [
                {
                  "name": "runtime",
                  "type": "example.runtime"
                }
              ]
            }
            "#,
        )
        .unwrap();

        let builtins = load_custom_builtins(&path).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!(builtins.global.len(), 1);
        assert_eq!(builtins.global[0].name, "runtime");
        assert_eq!(builtins.global[0].r#type, "example.runtime");
    }
}

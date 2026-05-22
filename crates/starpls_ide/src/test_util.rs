use starpls_bazel::decode_custom_builtins_json;
use starpls_common::Dialect;
use starpls_common::FileInfo;
use starpls_hir::Fixture;

use crate::Analysis;

pub(crate) fn custom_analysis_from_single_file(input: &str) -> (Analysis, Fixture) {
    let schema_id = "test-manifest.starpls.json#example".to_string();
    let (mut analysis, _) = Analysis::new_for_test();
    analysis.set_custom_builtin_defs(schema_id.clone(), custom_builtins());

    let mut fixture = Fixture::new(&mut analysis.db);
    fixture.add_file_with_options(
        &mut analysis.db,
        "main.star",
        input,
        Dialect::Standard,
        Some(FileInfo::Custom { schema: schema_id }),
    );

    (analysis, fixture)
}

fn custom_builtins() -> starpls_bazel::Builtins {
    decode_custom_builtins_json(
        r#"
        {
          "global": [
            {
              "name": "make_document",
              "doc": "Creates a document.",
              "callable": {
                "params": [
                  {
                    "name": "title",
                    "type": "string",
                    "doc": "Document title.",
                    "is_mandatory": true
                  }
                ],
                "return_type": "example.Document"
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
              "field": [
                {
                  "name": "decode",
                  "doc": "Decodes a payload.",
                  "callable": {
                    "params": [
                      {
                        "name": "payload",
                        "type": "string",
                        "doc": "Serialized payload.",
                        "is_mandatory": true
                      }
                    ],
                    "return_type": "example.Document"
                  }
                }
              ]
            },
            {
              "name": "example.Document",
              "field": [
                {
                  "name": "title",
                  "type": "string"
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
            },
            {
              "name": "example.Metadata",
              "field": [
                {
                  "name": "name",
                  "type": "string"
                }
              ]
            }
          ]
        }
        "#,
    )
    .unwrap()
}

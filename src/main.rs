use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use pulldown_cmark_to_cmark::cmark;
use std::process::Command;

fn compile(prql: &str) -> Result<String, prql_compiler::ErrorMessages> {
    anstream::ColorChoice::Never.write_global();
    prql_compiler::compile(prql, &prql_compiler::Options::default().no_signature())
}

enum PrqlBlockMode {
    Eval,
    Table,
    NoEval,
    NoTest,
    Error,
}

fn prql_block_mode(info: &pulldown_cmark::CowStr) -> Option<PrqlBlockMode> {
    match info.strip_prefix("prql ").unwrap_or_default() {
        "no-eval" => Some(PrqlBlockMode::NoEval),
        "no-test" => Some(PrqlBlockMode::NoTest),
        "error" => Some(PrqlBlockMode::Error),
        "table" => Some(PrqlBlockMode::Table),
        _ => Some(PrqlBlockMode::Eval),
    }
}

fn table_of_comparison(prql: &str, sql: &str) -> String {
    format!(
        r#"
<div class="comparison">

<div>

```prql title="PRQL"
{prql}
```

</div>

<div>

```sql title="SQL"
{sql}
```

</div>

</div>
"#,
        prql = prql.trim(),
        sql = sql.trim(),
    )
    .trim_start()
    .to_string()
}

fn table_of_error(prql: &str, message: &str) -> String {
    format!(
        r#"
<div class="comparison">

<div>

```prql title="PRQL"
{prql}
```

</div>

<div>

```text title="Error"
{message}
```

</div>

</div>
"#,
        prql = prql.trim(),
        message = message,
    )
    .trim_start()
    .to_string()
}

fn md_to_md(md: &str) -> String {
    let mut eval_prql = false;
    let mut eval_error = false;
    let mut eval_table = false;
    let parser = Parser::new(md).map(|event| match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
            if info.starts_with("prql") {
                match prql_block_mode(info) {
                    Some(PrqlBlockMode::Eval) => {
                        eval_prql = true;
                        Event::Text(pulldown_cmark::CowStr::Borrowed("\n"))
                    }
                    Some(PrqlBlockMode::Table) => {
                        eval_table = true;
                        Event::Text(pulldown_cmark::CowStr::Borrowed("\n"))
                    }
                    Some(PrqlBlockMode::Error) => {
                        eval_error = true;
                        Event::Text(pulldown_cmark::CowStr::Borrowed("\n"))
                    }
                    _ => Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
                        r#"prql title="PRQL""#.into(),
                    ))),
                }
            } else {
                event
            }
        }
        Event::End(Tag::CodeBlock(_)) => {
            if eval_prql | eval_table | eval_error {
                eval_prql = false;
                eval_table = false;
                eval_error = false;
                Event::Text(pulldown_cmark::CowStr::Borrowed("\n"))
            } else {
                event
            }
        }
        Event::Text(ref t) => {
            let prql = t.to_string();
            let result = compile(&prql);
            if eval_prql {
                let sql = compile(&prql).unwrap();
                Event::Html(table_of_comparison(&prql, &sql).into())
            } else if eval_table {
                let sql = compile(&prql).unwrap();
                let output = Command::new("duckdb")
                    .args(["-markdown", "-c", &sql])
                    .output()
                    .expect("faild");
                let table = String::from_utf8(output.stdout).unwrap();
                Event::Html(table.into())
            } else if eval_error {
                let error_message = match result {
                    Ok(sql) => {
                        unreachable!("Query was labeled to raise an error, but succeeded.\n{prql}\n\n{sql}\n\n");
                    }
                    Err(e) => ansi_to_html::convert_escaped(e.to_string().as_str()).unwrap(),
                };
                Event::Html(table_of_error(&prql, &error_message).into())
            } else {
                event
            }
        }
        _ => event,
    });

    let mut buf = String::new();
    cmark(parser, &mut buf).unwrap();

    buf
}

fn main() {
    let s = r#"
<!-- ---
# YAML front matter will be supported in the next version of pulldown-cmark.
# https://github.com/raphlinus/pulldown-cmark/pull/641
title: Introduction
sidebar_position: 1
slug: /
--- -->

Some code blocks.

```prql
from a
```

```prql no-eval
from b
```

```prql error
from b d
```

```prql table
prql target:sql.duckdb

from [
  {time_id=1, value=15},
  {time_id=2, value=11},
  {time_id=3, value=16},
  {time_id=4, value=9},
  {time_id=7, value=20},
  {time_id=8, value=22},
]
window rows:-2..0 (
  sort time_id
  derive {sma3rows = average value}
)
window range:-2..0 (
  sort time_id
  derive {sma3range = average value}
)
```
"#;

    println!("{}", md_to_md(s));
}

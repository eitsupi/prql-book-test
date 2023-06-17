use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use pulldown_cmark_to_cmark::cmark;

fn compile(prql: &str) -> Result<String, prql_compiler::ErrorMessages> {
    anstream::ColorChoice::Never.write_global();
    prql_compiler::compile(prql, &prql_compiler::Options::default().no_signature())
}

enum PrqlBlockMode {
    Eval,
    NoEval,
    NoTest,
}

fn prql_block_mode(info: &pulldown_cmark::CowStr) -> Option<PrqlBlockMode> {
    match info.strip_prefix("prql ").unwrap_or_default() {
        "no-eval" => Some(PrqlBlockMode::NoEval),
        "no-test" => Some(PrqlBlockMode::NoTest),
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

fn main() {
    let s = r#"
Some code blocks.

```prql
from a
```

```prql no-eval
from b
```
"#;
    let mut eval_prql = false;
    let parser = Parser::new(s).map(|event| match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
            if info.starts_with("prql") {
                match prql_block_mode(info) {
                    Some(PrqlBlockMode::Eval) => {
                        eval_prql = true;
                        Event::Text(pulldown_cmark::CowStr::Borrowed("\n"))
                    }
                    _ => Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(r#"prql title="PRQL""#.into()))),
                }
            } else {
                event
            }
        }
        Event::End(Tag::CodeBlock(_)) => {
            if eval_prql {
                eval_prql = false;
                Event::Text(pulldown_cmark::CowStr::Borrowed("\n"))
            } else {
                event
            }
        }
        Event::Text(ref t) => {
            if eval_prql {
                let prql = t.to_string();
                let sql = compile(&prql).unwrap();
                Event::Html(table_of_comparison(&prql, &sql).into())
            } else {
                event
            }
        }
        _ => event,
    });

    let mut buf = String::new();
    cmark(parser, &mut buf).unwrap();
    println!("{}", buf);
}

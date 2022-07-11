use crate::compile::compile;
use crate::errors::{Error, Kind};
use crate::spec::{Content, Object, Reference, SchemaExpr, Spec, Uri, UriSegment};
use crate::{Locator, ModuleSet, Program};
use oal_syntax::{atom, parse};

fn eval(code: &str) -> anyhow::Result<Spec> {
    let loc = Locator::try_from("test:main")?;
    let mut mods = ModuleSet::new(loc.clone());
    let prg: Program = parse(code)?;
    let prg = compile(&mods, &loc, prg)?;
    mods.insert(loc, prg);

    let spec = Spec::try_from(&mods)?;

    anyhow::Ok(spec)
}

#[test]
fn uri_pattern() {
    let uri = Uri {
        path: vec![UriSegment::Literal("".into())],
        params: None,
        example: None,
    };

    assert_eq!(uri.pattern(), "/");
}

#[test]
fn evaluate_simple() -> anyhow::Result<()> {
    let code = r#"
        # description: "some record"
        let r = {};
        res / ( put : <r> -> <r> );
    "#;

    let s = eval(code)?;

    assert_eq!(s.rels.len(), 1);

    let (i, p) = s.rels.iter().next().unwrap();

    assert_eq!(i, "/");
    assert_eq!(p.uri.path.len(), 1);
    assert_eq!(*p.uri.path.first().unwrap(), UriSegment::Literal("".into()));

    if let Some(x) = &p.xfers[atom::Method::Put] {
        let d = x.domain.schema.as_ref().unwrap();
        assert_eq!(d.expr, SchemaExpr::Object(Object::default()));
        assert_eq!(d.desc, Some("some record".to_owned()));
        let r = x.ranges.values().next().unwrap().schema.as_ref().unwrap();
        assert_eq!(r.expr, SchemaExpr::Object(Object::default()));
        assert_eq!(r.desc, Some("some record".to_owned()));
    } else {
        panic!("expected transfer on HTTP PUT");
    }

    anyhow::Ok(())
}

#[test]
fn evaluate_content() -> anyhow::Result<()> {
    let code = r#"
        let r = {};
        res / ( put : r -> <r> );
    "#;

    let s = eval(code)?;

    assert_eq!(s.rels.len(), 1);

    anyhow::Ok(())
}

#[test]
fn evaluate_ranges() -> anyhow::Result<()> {
    let code = r#"
        res / ( get -> <status=200,{}> :: <status=500,media="text/plain",headers={},{}> );
    "#;

    let spec = eval(code)?;

    let rel = spec.rels.values().next().expect("expected relation");

    let xfer = rel.xfers[atom::Method::Get]
        .as_ref()
        .expect("expected get transfer");

    assert_eq!(xfer.ranges.len(), 2);

    let cnt: &Content = xfer.ranges.last().unwrap().1;

    assert_eq!(
        cnt.status,
        Some(atom::HttpStatus::Code(500.try_into().unwrap()))
    );
    assert_eq!(cnt.media, Some("text/plain".to_owned()));
    assert_eq!(cnt.headers, Some(Object::default()));

    anyhow::Ok(())
}

#[test]
fn evaluate_invalid_status() -> anyhow::Result<()> {
    let code = r#"
        res / ( get -> <status=999,{}> );
    "#;

    assert_eq!(
        eval(code)
            .expect_err(format!("expected error evaluating: {}", code).as_str())
            .downcast_ref::<Error>()
            .expect("expected compiler error")
            .kind,
        Kind::InvalidSyntax
    );

    anyhow::Ok(())
}

#[test]
fn evaluate_reference() -> anyhow::Result<()> {
    let code = r#"
        let @a = {};
        res / ( get -> @a );
    "#;

    let spec = eval(code)?;

    let (name, ref_) = spec.refs.iter().next().expect("expected reference");

    assert_eq!(name.as_ref(), "@a");

    match ref_ {
        Reference::Schema(s) => match s.expr {
            SchemaExpr::Object(_) => {}
            _ => panic!("expected object expression"),
        },
    }

    let rel = spec.rels.values().next().expect("expected relation");

    let xfer = rel.xfers[atom::Method::Get]
        .as_ref()
        .expect("expected get transfer");

    let range = xfer
        .ranges
        .values()
        .next()
        .unwrap()
        .schema
        .as_ref()
        .unwrap();
    assert_eq!(range.expr, SchemaExpr::Ref("@a".into()));

    anyhow::Ok(())
}

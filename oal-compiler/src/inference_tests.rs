use crate::errors::{Error, Kind};
use crate::expr::TypedExpr;
use crate::inference::{constrain, substitute, tag_type, InferenceSet, TagSeq};
use crate::node::NodeRef;
use crate::scan::Scan;
use crate::scope::Env;
use crate::tag::{Tag, Tagged};
use crate::transform::Transform;
use crate::Program;
use oal_syntax::ast::{AsRefNode, Expr, Lambda, Statement};
use oal_syntax::parse;

#[test]
fn tag_var_decl() {
    let mut d: Program = parse("let id1 = num;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    d.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type)
        .expect("tagging failed");

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        assert_eq!(decl.expr.unwrap_tag(), Tag::Primitive);
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn tag_array_decl() {
    let mut d: Program = parse("let id1 = [num];").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    d.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type)
        .expect("tagging failed");

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        assert_eq!(decl.expr.unwrap_tag(), Tag::Array);
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn tag_lambda_decl() {
    let mut d: Program = parse("let f x y z = num;").expect("parsing failed");

    d.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type)
        .expect("tagging failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        assert_eq!(decl.name.as_ref(), "f");
        assert_eq!(decl.expr.unwrap_tag(), Tag::Var(3));
        if let Expr::Lambda(Lambda { bindings, .. }) = decl.expr.as_node().as_expr() {
            let tags: Vec<_> = bindings
                .iter()
                .filter_map(|a| match a.tag() {
                    Some(Tag::Var(n)) => Some(*n),
                    _ => None,
                })
                .collect();
            assert_eq!(tags, vec![0, 1, 2]);
        } else {
            panic!("expected lambda expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn tag_not_in_scope() {
    let mut d: Program = parse("let a = f {};").expect("parsing failed");

    let r = d.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type);

    if let Err(e) = r {
        assert_eq!(e.kind, Kind::NotInScope);
    } else {
        panic!("expected error");
    }
}

#[test]
fn constraint_var() {
    let code = r#"
        let id1 = {} & {};
        let id2 = id1 | {};
    "#;
    let mut d: Program = parse(code).expect("parsing failed");

    d.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type)
        .expect("tagging failed");

    let cnt = &mut InferenceSet::new();

    d.scan(cnt, &mut Env::new(None), &mut constrain)
        .expect("constraining failed");

    assert_eq!(cnt.len(), 8);
}

#[test]
fn constraint_lambda() {
    let mut d: Program = parse("let f x y z = num;").expect("parsing failed");

    d.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type)
        .expect("tagging failed");

    let cnt = &mut InferenceSet::new();

    d.scan(cnt, &mut Env::new(None), &mut constrain)
        .expect("constraining failed");

    assert_eq!(cnt.len(), 2);
}

#[test]
fn unify_simple() {
    let mut c = InferenceSet::new();

    c.push(Tag::Var(0), Tag::Primitive, None);
    c.push(Tag::Var(2), Tag::Var(1), None);
    c.push(Tag::Var(1), Tag::Var(0), None);

    let u = c.unify().expect("unification failed");

    let t = u.substitute(&Tag::Var(2));

    assert_eq!(t, Tag::Primitive);
}

fn check_tags(
    _acc: &mut (),
    _env: &mut Env<TypedExpr>,
    node: NodeRef<TypedExpr>,
) -> crate::errors::Result<()> {
    match node {
        NodeRef::Expr(e) => match e.tag() {
            None => Err(Error::new(Kind::Unknown, "missing tag").with(e)),
            Some(Tag::Var(_)) => Err(Error::new(Kind::Unknown, "remaining tag variable").with(e)),
            Some(_) => Ok(()),
        },
        _ => Ok(()),
    }
}

#[test]
fn unify_lambda() {
    let code = r#"
        let f x y z = num;
        let a = f num {} uri;
    "#;
    let mut prg: Program = parse(code).expect("parsing failed");

    prg.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type)
        .expect("tagging failed");

    let cnt = &mut InferenceSet::new();

    prg.scan(cnt, &mut Env::new(None), &mut constrain)
        .expect("constraining failed");

    let subst = &mut cnt.unify().expect("unification failed");

    prg.transform(subst, &mut Env::new(None), &mut substitute)
        .expect("substitution failed");

    prg.scan(&mut (), &mut Env::new(None), &mut check_tags)
        .expect("substitution incomplete");
}

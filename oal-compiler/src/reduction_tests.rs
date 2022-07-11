use crate::errors::{Error, Kind};
use crate::expr::TypedExpr;
use crate::inference::{constrain, substitute, tag_type, InferenceSet, TagSeq};
use crate::node::NodeRef;
use crate::reduction::reduce;
use crate::scan::Scan;
use crate::scope::Env;
use crate::transform::Transform;
use crate::Program;
use oal_syntax::ast::{AsRefNode, Expr, Operator, Statement};
use oal_syntax::atom::Primitive;
use oal_syntax::parse;

/// Evaluates a program.
fn eval(code: &str) -> anyhow::Result<Program> {
    let mut prg = parse(code)?;

    prg.transform(&mut TagSeq::default(), &mut Env::new(None), &mut tag_type)?;

    let cnt = &mut InferenceSet::new();

    prg.scan(cnt, &mut Env::new(None), &mut constrain)?;

    let subst = &mut cnt.unify()?;

    prg.transform(subst, &mut Env::new(None), &mut substitute)?;

    prg.transform(&mut (), &mut Env::new(None), &mut reduce)?;

    prg.scan(&mut (), &mut Env::new(None), &mut check_free_vars)?;

    anyhow::Ok(prg)
}

/// Checks that no free variable by-value remains.
fn check_free_vars(
    _acc: &mut (),
    env: &mut Env<TypedExpr>,
    node: NodeRef<TypedExpr>,
) -> crate::errors::Result<()> {
    match node {
        NodeRef::Expr(e) => match e.as_node().as_expr() {
            Expr::Var(var) if var.is_value() => match env.lookup(var) {
                None => Err(Error::new(Kind::NotInScope, "").with(e)),
                Some(val) => match val.as_node().as_expr() {
                    Expr::Binding(_) => Ok(()),
                    _ => Err(Error::new(Kind::Unknown, "remaining free variable").with(e)),
                },
            },
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

#[test]
fn reduce_application() {
    let code = r#"
        let b = str;
        let g x = b;
        let b = bool;
        let f x = x | num | g x;
        let a = f bool;
    "#;
    let prg = eval(code).expect("evaluation failed");

    match prg.stmts.iter().nth(4).unwrap() {
        Statement::Decl(d) => {
            assert_eq!(d.name.as_ref(), "a");
            match d.expr.as_node().as_expr() {
                Expr::Op(o) => {
                    assert_eq!(o.op, Operator::Sum);
                    let mut i = o.exprs.iter();
                    assert_eq!(
                        *i.next().unwrap().as_node().as_expr(),
                        Expr::Prim(Primitive::Boolean)
                    );
                    assert_eq!(
                        *i.next().unwrap().as_node().as_expr(),
                        Expr::Prim(Primitive::Number)
                    );
                    assert_eq!(
                        *i.next().unwrap().as_node().as_expr(),
                        Expr::Prim(Primitive::String)
                    );
                }
                _ => panic!("expected operation"),
            }
        }
        _ => panic!("expected declaration"),
    }
}

#[test]
fn reduce_reference() {
    let code = r#"
        let @a = {};
        let b = @a;
    "#;
    let prg = eval(code).expect("evaluation failed");

    match prg.stmts.iter().nth(1).unwrap() {
        Statement::Decl(d) => {
            assert_eq!(d.name.as_ref(), "b");
            match d.expr.as_node().as_expr() {
                Expr::Var(v) => {
                    assert!(v.is_reference());
                    assert_eq!(v.as_ref(), "@a");
                }
                _ => panic!("expected variable"),
            }
        }
        _ => panic!("expected declaration"),
    }
}

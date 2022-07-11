use crate::errors::Result;
use crate::locator::Locator;
use crate::module::load;
use crate::{compile, Program};
use oal_syntax::ast::{AsRefNode, Expr, Operator, Statement};
use oal_syntax::parse;

#[test]
fn compile_module() {
    let module = &Locator::try_from("test:module.oal").unwrap();
    let main = &Locator::try_from("test:main.oal").unwrap();
    let loader = |m: &Locator| -> Result<Program> {
        if m == module {
            Ok(parse("let f x = x | num;").expect("parsing failed"))
        } else if m == main {
            Ok(parse(r#"use "test:module.oal"; let id = f str;"#).expect("parsing failed"))
        } else {
            unreachable!()
        }
    };
    let mods = load(main, loader, compile).expect("loading failed");

    assert_eq!(mods.len(), 2);

    if let Statement::Decl(d) = mods.main().stmts.last().expect("expected statement") {
        if let Expr::Op(op) = d.expr.as_node().as_expr() {
            assert_eq!(op.op, Operator::Sum);
        } else {
            panic!("expected operation")
        }
    } else {
        panic!("expected declaration")
    }
}

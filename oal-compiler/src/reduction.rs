use crate::errors::{Error, Kind, Result};
use crate::node::NodeMut;
use crate::scope::Env;
use crate::transform::Transform;
use oal_syntax::ast::{AsExpr, Expr};

/// Associative binary operation for expression reduction.
pub trait Semigroup: Sized {
    fn combine(&mut self, with: Self) {
        *self = with;
    }
}

/// Visits an abstract syntax tree to reduce expressions.
pub fn reduce<T>(_acc: &mut (), env: &mut Env<T>, node_ref: NodeMut<T>) -> Result<()>
where
    T: AsExpr + Semigroup,
{
    if let NodeMut::Expr(expr) = node_ref {
        let node = expr.as_node_mut();
        let span = node.span;
        match node.as_expr_mut() {
            Expr::Var(var) if var.is_value() => match env.lookup(var) {
                None => Err(Error::new(Kind::NotInScope, "").with(expr)),
                Some(val) => {
                    match val.as_node().as_expr() {
                        Expr::Binding(_) => {}
                        _ => expr.combine(val.clone()),
                    };
                    Ok(())
                }
            },
            Expr::App(application) => match env.lookup(&application.name) {
                None => Err(Error::new(Kind::NotAFunction, "").with(expr)),
                Some(val) => {
                    if let Expr::Lambda(lambda) = val.as_node().as_expr() {
                        let app_env = &mut Env::new(None);
                        for (binding, arg) in lambda.bindings.iter().zip(application.args.iter()) {
                            if let Expr::Binding(name) = binding.as_node().as_expr() {
                                app_env.declare(name.clone(), arg.clone())
                            } else {
                                unreachable!()
                            }
                        }
                        let mut app = lambda.body.as_ref().clone();
                        app.as_node_mut()
                            .as_expr_mut()
                            .transform(&mut (), app_env, &mut reduce)?;
                        expr.combine(app);
                        Ok(())
                    } else {
                        Err(Error::new(Kind::NotAFunction, "").with(expr))
                    }
                }
            },
            _ => Ok(()),
        }
        .map_err(|err| err.at(span))
    } else {
        Ok(())
    }
}

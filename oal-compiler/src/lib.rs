use std::collections::HashMap;
use std::rc::Rc;

use oal_syntax::ast::*;

type Env = HashMap<Ident, TypeAny>;

#[derive(Debug)]
enum List<T> {
    Nil,
    Cons(T, Rc<List<T>>),
}

impl<T: Eq> List<T> {
    fn contains(&self, x: &T) -> bool {
        match self {
            Self::Nil => false,
            Self::Cons(h, t) => x == h || t.contains(x),
        }
    }
}

type Path = Rc<List<Ident>>;

#[derive(Debug, Clone)]
struct EvalError {
    msg: String,
}

impl EvalError {
    fn new(msg: &str) -> EvalError {
        EvalError { msg: msg.into() }
    }
}

impl From<&str> for EvalError {
    fn from(msg: &str) -> Self {
        Self::new(msg)
    }
}

type Result<T> = std::result::Result<T, EvalError>;

#[derive(PartialEq, Clone, Copy, Debug)]
enum TypeTag {
    Prim,
    Uri,
    Block,
    Unknown,
}

fn welltype(expr: &TypeAny) -> Result<TypeTag> {
    match expr {
        TypeAny::Prim(_) => Ok(TypeTag::Prim),
        TypeAny::Rel(TypeRel {
            uri,
            methods: _methods,
            range,
        }) => {
            let uri = welltype(uri).and_then(|t| {
                if let TypeTag::Uri = t {
                    Ok(t)
                } else {
                    Err("expected uri as relation base".into())
                }
            });
            let range = welltype(range).and_then(|t| {
                if let TypeTag::Block = t {
                    Ok(t)
                } else {
                    Err("expected block as range".into())
                }
            });

            uri.and_then(|_| range.and_then(|_| Ok(TypeTag::Unknown)))
        }
        TypeAny::Uri(TypeUri { spec }) => {
            let r: Result<Vec<_>> = spec
                .iter()
                .map(|s| match s {
                    UriSegment::Template(Prop(_, e)) => welltype(e).and_then(|t| {
                        if let TypeTag::Prim = t {
                            Ok(())
                        } else {
                            Err("expected prim as uri template property".into())
                        }
                    }),
                    UriSegment::Literal(_) => Ok(()),
                })
                .collect();

            r.map(|_| TypeTag::Uri)
        }
        TypeAny::Sum(TypeSum(sum)) => {
            let sum: Result<Vec<_>> = sum.iter().map(|e| welltype(e)).collect();

            sum.map(|sum| {
                sum.iter()
                    .reduce(|a, b| if a == b { a } else { &TypeTag::Unknown })
                    .unwrap_or(&TypeTag::Unknown)
                    .clone()
            })
        }
        TypeAny::Var(_) => Err("unresolved variable".into()),
        TypeAny::Join(TypeJoin(join)) => {
            let r: Result<Vec<_>> = join
                .iter()
                .map(|e| {
                    welltype(e).and_then(|t| {
                        if t == TypeTag::Block {
                            Ok(())
                        } else {
                            Err("expected block as join element".into())
                        }
                    })
                })
                .collect();

            r.map(|_| TypeTag::Block)
        }
        TypeAny::Block(_) => Ok(TypeTag::Block),
    }
}

fn resolve(env: &Env, from: Path, expr: &TypeAny) -> Result<TypeAny> {
    match expr {
        TypeAny::Var(v) => {
            if from.contains(v) {
                Err("cycle detected".into())
            } else {
                let path = Rc::new(List::Cons(v.clone(), from));
                match env.get(v) {
                    None => Err("unknown identifier".into()),
                    Some(e) => resolve(env, path, e),
                }
            }
        }
        TypeAny::Prim(_) => Ok(expr.clone()),
        TypeAny::Rel(TypeRel {
            uri,
            methods,
            range,
        }) => {
            let uri = resolve(env, from.clone(), uri);
            let methods = methods.clone();
            let range = resolve(env, from, range);

            uri.and_then(|uri| {
                range.and_then(|range| {
                    Ok(TypeAny::Rel(TypeRel {
                        uri: Box::new(uri),
                        methods,
                        range: Box::new(range),
                    }))
                })
            })
        }
        TypeAny::Uri(TypeUri { spec }) => {
            let spec: Result<Vec<_>> = spec
                .iter()
                .map(|s| match s {
                    UriSegment::Literal(_) => Ok(s.clone()),
                    UriSegment::Template(Prop(id, e)) => resolve(env, from.clone(), e)
                        .map(|e| UriSegment::Template(Prop(id.clone(), e))),
                })
                .collect();

            spec.map(|spec| TypeAny::Uri(TypeUri { spec }))
        }
        TypeAny::Block(TypeBlock(props)) => {
            let props: Result<Vec<_>> = props
                .iter()
                .map(|Prop(p, e)| resolve(env, from.clone(), e).map(|e| Prop(p.clone(), e)))
                .collect();

            props.map(|props| TypeAny::Block(TypeBlock(props)))
        }
        TypeAny::Sum(TypeSum(sum)) => {
            let sum: Result<Vec<_>> = sum.iter().map(|e| resolve(env, from.clone(), e)).collect();

            sum.map(|sum| TypeAny::Sum(TypeSum(sum)))
        }
        TypeAny::Join(TypeJoin(join)) => {
            let join: Result<Vec<_>> = join.iter().map(|e| resolve(env, from.clone(), e)).collect();

            join.map(|join| TypeAny::Join(TypeJoin(join)))
        }
    }
}

fn environment(d: &Doc) -> Env {
    d.stmts
        .iter()
        .flat_map(|s| match s {
            Stmt::Decl { var, expr } => Some((var.clone(), expr.clone())),
            _ => None,
        })
        .collect()
}

pub fn visit(d: &Doc) {
    let env = environment(&d);

    let resources: Vec<_> = d
        .stmts
        .iter()
        .flat_map(|s| match s {
            Stmt::Res { rel } => Some(rel),
            _ => None,
        })
        .map(|e| resolve(&env, Rc::new(List::Nil), e).and_then(|e| welltype(&e).map(|t| (e, t))))
        .collect();

    println!("{:#?}", resources)
}
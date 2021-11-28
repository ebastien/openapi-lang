use crate::{Pair, Rule};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum TypeAny {
    Prim(TypePrim),
    Rel(TypeRel),
    Uri(TypeUri),
    Join(TypeJoin),
    Block(TypeBlock),
    Sum(TypeSum),
    Var(Ident),
}

#[derive(Clone, Debug)]
pub struct Doc {
    pub stmts: Vec<Stmt>,
}

impl From<Pair<'_>> for Doc {
    fn from(p: Pair) -> Self {
        let stmts = p
            .into_inner()
            .flat_map(|p| match p.as_rule() {
                Rule::stmt => Some(p.into()),
                _ => None,
            })
            .collect();
        Doc { stmts }
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Decl { var: Ident, expr: TypeAny },
    Res { rel: TypeAny },
}

impl From<Pair<'_>> for Stmt {
    fn from(p: Pair) -> Self {
        let p = p.into_inner().next().unwrap();
        match p.as_rule() {
            Rule::decl => {
                let mut p = p.into_inner();
                let var = p.nth(1).unwrap().as_str().into();
                let expr = p.next().unwrap().into();
                Stmt::Decl { var, expr }
            }
            Rule::res => Stmt::Res {
                rel: p.into_inner().nth(1).unwrap().into(),
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Method {
    Get,
    Put,
}

impl From<Pair<'_>> for Method {
    fn from(p: Pair) -> Self {
        match p.into_inner().next().unwrap().as_rule() {
            Rule::get_kw => Method::Get,
            Rule::put_kw => Method::Put,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeRel {
    pub uri: Box<TypeAny>,
    pub methods: Vec<Method>,
    pub range: Box<TypeAny>,
}

impl From<Pair<'_>> for TypeRel {
    fn from(p: Pair) -> Self {
        let mut inner = p.into_inner();

        let uri: Box<_> = TypeAny::from(inner.next().unwrap()).into();

        let methods: Vec<_> = inner
            .next()
            .unwrap()
            .into_inner()
            .map(|p| p.into())
            .collect();

        let range: Box<_> = TypeAny::from(inner.next().unwrap()).into();

        TypeRel {
            uri,
            methods,
            range,
        }
    }
}

#[derive(Clone, Debug)]
pub enum UriSegment {
    Literal(Rc<str>),
    Template(Prop),
}

#[derive(Clone, Debug)]
pub struct TypeUri {
    pub spec: Vec<UriSegment>,
}

impl From<Pair<'_>> for TypeUri {
    fn from(p: Pair) -> Self {
        let mut p = p.into_inner();
        p.next();
        let spec: Vec<_> = p
            .next()
            .map(|p| {
                p.into_inner()
                    .map(|p| match p.as_rule() {
                        Rule::uri_tpl => {
                            UriSegment::Template(p.into_inner().next().unwrap().into())
                        }
                        Rule::uri_lit => {
                            UriSegment::Literal(p.into_inner().next().unwrap().as_str().into())
                        }
                        _ => unreachable!(),
                    })
                    .collect()
            })
            .unwrap_or(vec![]);
        TypeUri { spec }
    }
}

pub type Ident = Rc<str>;

#[derive(Clone, Debug)]
pub struct Prop(pub Ident, pub TypeAny);

impl From<Pair<'_>> for Prop {
    fn from(p: Pair) -> Self {
        let mut inner = p.into_inner();
        let id = inner.next().unwrap().as_str().into();
        let expr = inner.next().unwrap().into();
        Prop(id, expr)
    }
}

#[derive(Clone, Debug)]
pub struct TypeBlock(pub Vec<Prop>);

impl From<Pair<'_>> for TypeBlock {
    fn from(p: Pair) -> Self {
        TypeBlock(p.into_inner().map(|p| p.into()).collect())
    }
}

#[derive(Clone, Debug)]
pub struct TypeJoin(pub Vec<TypeAny>);

impl From<Pair<'_>> for TypeJoin {
    fn from(p: Pair) -> Self {
        TypeJoin(p.into_inner().map(|p| p.into()).collect())
    }
}

#[derive(Clone, Debug)]
pub struct TypeSum(pub Vec<TypeAny>);

impl From<Pair<'_>> for TypeSum {
    fn from(p: Pair) -> Self {
        let types = p
            .into_inner()
            .map(|p| p.into_inner().next().unwrap().into())
            .collect();
        TypeSum(types)
    }
}

#[derive(Clone, Debug)]
pub enum TypePrim {
    Num,
    Str,
    Bool,
}

impl From<Pair<'_>> for TypePrim {
    fn from(p: Pair) -> Self {
        match p.into_inner().next().unwrap().as_rule() {
            Rule::num_kw => TypePrim::Num,
            Rule::str_kw => TypePrim::Str,
            Rule::bool_kw => TypePrim::Bool,
            _ => unreachable!(),
        }
    }
}

impl From<Pair<'_>> for TypeAny {
    fn from(p: Pair<'_>) -> Self {
        match p.as_rule() {
            Rule::prim_type => TypeAny::Prim(p.into()),
            Rule::rel_type => TypeAny::Rel(p.into()),
            Rule::uri_type => TypeAny::Uri(p.into()),
            Rule::join_type => match TypeJoin::from(p) {
                TypeJoin(join) if join.len() == 1 => join.first().unwrap().clone(),
                t @ _ => TypeAny::Join(t),
            },
            Rule::sum_type => match TypeSum::from(p) {
                TypeSum(sum) if sum.len() == 1 => sum.first().unwrap().clone(),
                t @ _ => TypeAny::Sum(t),
            },
            Rule::block_type => TypeAny::Block(p.into()),
            Rule::ident => TypeAny::Var(p.as_str().into()),
            _ => unreachable!(),
        }
    }
}
